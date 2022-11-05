use std::collections::HashMap;
use std::fs::read_to_string;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Result};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, get_service};
use axum::{Extension, Json, Router};
use chrono::{Duration, Local};
use clap::Parser;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::{Acquire, Row, SqlitePool};
use tokio::try_join;
use tower_http::services::ServeDir;

#[derive(Deserialize, Debug)]
struct Chore {
    description: String,
    frequency: String,
}

const fn one_day() -> StdDuration {
    StdDuration::from_secs(86400)
}

const fn one_hour() -> StdDuration {
    StdDuration::from_secs(3600)
}

const fn default_port() -> u16 {
    4040
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "default_port")]
    port: u16,
    chores: HashMap<String, Chore>,
    #[serde(with = "humantime_serde")]
    overdue_time: StdDuration,
    #[serde(with = "humantime_serde", default = "one_day")]
    lookahead_time: StdDuration,
    #[serde(with = "humantime_serde", default = "one_hour")]
    check_interval: StdDuration,
}

impl Config {
    fn from_path(path: &str) -> Result<Arc<Self>> {
        let contents = read_to_string(path)?;

        Ok(Arc::new(serde_json::from_str(&contents)?))
    }
}

/// Chores webserver
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config file to load from
    #[arg(long, default_value = "config.json")]
    config_path: String,
}

async fn update_chores(pool: Arc<SqlitePool>, config: Arc<Config>) -> Result<()> {
    loop {
        let now = Local::now();
        let lookahead = now + Duration::from_std(config.lookahead_time)?;
        let overdue_duration = Duration::from_std(config.overdue_time)?;

        let mut conn = pool.acquire().await?;
        let mut txn = conn.begin().await?;

        let mut added_chores = 0;

        sqlx::query!(
            r#"
            UPDATE `chores`
            SET
                `status` = 'missed'
            WHERE
                `expected_completion_time` < STRFTIME('%s', 'now', 'localtime')
                AND `status` = 'assigned'
            "#,
        )
        .execute(&mut txn)
        .await?;

        for (title, chore) in config.chores.iter() {
            let chore_title = title.to_string();

            let schedule: Schedule = chore.frequency.parse()?;
            for next_time in schedule.upcoming(Local) {
                if next_time > lookahead {
                    break;
                }

                added_chores += 1;

                let expected_timestamp = next_time.timestamp();
                let overdue_timestamp = (next_time + overdue_duration).timestamp();

                sqlx::query!(
                    r#"
                    INSERT OR IGNORE INTO `chores`
                    (
                        `title`,
                        `expected_completion_time`,
                        `overdue_time`
                    )
                    VALUES
                    (
                        ?1,
                        ?2,
                        ?3
                    )
                    "#,
                    chore_title,
                    expected_timestamp,
                    overdue_timestamp,
                )
                .execute(&mut txn)
                .await?;
            }
        }

        txn.commit().await?;

        tracing::debug!("Added {} chore(s)", added_chores);

        tokio::time::sleep(config.check_interval).await
    }
}

async fn handle_error(_err: std::io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
enum Status {
    Assigned,
    Completed,
    Missed,
}

impl FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "assigned" => Ok(Status::Assigned),
            "completed" => Ok(Status::Completed),
            "missed" => Ok(Status::Missed),
            _ => Err(anyhow!("Unknown status \"{}\"", value)),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct ApiChore {
    title: String,
    description: String,
    expected_completion_time: i32,
    overdue: bool,
    status: Status,
}

#[derive(Serialize, Debug, Clone)]
struct ApiListChoresResponse {
    success: bool,
    error: Option<String>,
    chores: Vec<ApiChore>,
}

#[derive(Debug, Deserialize)]
struct ListChoresParams {
    lookback_days: Option<i64>,
}

async fn list_chores_impl(
    params: ListChoresParams,
    pool: Arc<SqlitePool>,
    config: Arc<Config>,
) -> Result<Vec<ApiChore>> {
    let lookback_days = params.lookback_days.unwrap_or(1);
    if lookback_days < 1 {
        return Err(anyhow!("Refusing to look back less than one day"));
    }
    let lookback_timestamp = (Local::now() - Duration::days(lookback_days)).timestamp();

    let rows = sqlx::query(
        r#"
        SELECT
            `title`,
            CAST(`expected_completion_time` AS INTEGER) AS `expected_completion_time`,
            STRFTIME('%s', 'now', 'localtime') > `overdue_time` AS `overdue`,
            `status`
        FROM `chores`
        WHERE CAST(`expected_completion_time` AS INTEGER) > ?1
        ORDER BY `expected_completion_time` ASC
        "#,
    )
    .bind(lookback_timestamp)
    .fetch_all(&*pool)
    .await?;

    let mut return_chores = Vec::new();
    for row in rows {
        let title = match row.try_get("title") {
            Ok(title) => title,
            Err(_) => {
                tracing::warn!("Chore missing title");
                continue;
            }
        };

        let description = match config.chores.get(&title) {
            Some(c) => c.description.clone(),
            None => {
                tracing::warn!("Chore \"{}\" not found in config", title);
                continue;
            }
        };

        let expected_completion_time = match row.try_get("expected_completion_time") {
            Ok(time) => time,
            Err(_) => {
                tracing::warn!("No expected completion time found for chore \"{}\"", title);
                continue;
            }
        };

        let overdue = match row.try_get::<i32, &str>("overdue") {
            Ok(overdue) => overdue == 1,
            Err(_) => {
                tracing::warn!("No overdue information found for chore \"{}\"", title);
                continue;
            }
        };

        let status = match row.try_get::<&str, &str>("status") {
            Ok(status_str) => match status_str.parse::<Status>() {
                Ok(status) => status,
                Err(_) => {
                    tracing::warn!("Unknown status \"{}\" for chore \"{}\"", status_str, title);
                    continue;
                }
            },
            Err(_) => {
                tracing::warn!("No status found for chore \"{}\"", title);
                continue;
            }
        };

        return_chores.push(ApiChore {
            title: title,
            description,
            expected_completion_time,
            overdue,
            status,
        });
    }

    Ok(return_chores)
}

async fn list_chores(
    Query(params): Query<ListChoresParams>,
    Extension(pool): Extension<Arc<SqlitePool>>,
    Extension(config): Extension<Arc<Config>>,
) -> Json<ApiListChoresResponse> {
    match list_chores_impl(params, pool, config).await {
        Ok(chores) => Json(ApiListChoresResponse {
            success: true,
            chores,
            error: None,
        }),
        Err(e) => Json(ApiListChoresResponse {
            success: false,
            chores: Vec::new(),
            error: Some(format!("failed to fetch chores: {}", e)),
        }),
    }
}

async fn serve(pool: Arc<SqlitePool>, config: Arc<Config>) -> Result<()> {
    let serve_dir = get_service(ServeDir::new("dist")).handle_error(handle_error);

    let app = Router::new()
        .route("/", get(|| async { "Hi from /" }))
        .nest("/dist", serve_dir.clone())
        .route("/api/chores", get(list_chores))
        .layer(Extension(pool))
        .layer(Extension(config.clone()));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        std::process::exit(1);
    }));

    let args = Args::parse();

    let config = Config::from_path(&args.config_path)?;

    tracing_subscriber::fmt::init();

    let pool = Arc::new(SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?);
    sqlx::migrate!().run(&*pool).await?;

    try_join!(
        update_chores(pool.clone(), config.clone()),
        serve(pool.clone(), config.clone()),
    )?;

    Ok(())
}
