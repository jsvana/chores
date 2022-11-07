use std::collections::HashMap;
use std::fs::read_to_string;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{anyhow, Result};
use axum::body;
use axum::body::Full;
use axum::extract::{Form, Query};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, get_service, post};
use axum::{Extension, Json, Router};
use chrono::{Duration, Local, TimeZone};
use clap::Parser;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::{Acquire, Row, SqlitePool};
use tokio::try_join;
use tower_http::services::ServeDir;

const INDEX_PATH: &'static str = "./assets/html/index.html";

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

        let row = sqlx::query(
            r#"
            SELECT
                CAST(`update_timestamp` AS INTEGER) AS `update_timestamp`
            FROM `updates`
            ORDER BY `update_timestamp` DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut txn)
        .await?;

        let last_update = match row {
            Some(row) => row
                .try_get("update_timestamp")
                .ok()
                .unwrap_or(now.timestamp()),
            None => now.timestamp(),
        };
        let last_update_date = Local.timestamp(last_update, 0);

        sqlx::query!(
            r#"
            UPDATE `chores`
            SET `status` = 'missed'
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
            for next_time in schedule.after(&last_update_date) {
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

        sqlx::query!(
            r#"
            INSERT OR IGNORE INTO `updates`
            (
                `update_timestamp`
            )
            VALUES
            (
                ?1
            )
            "#,
            last_update,
        )
        .execute(&mut txn)
        .await?;

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
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
enum ApiStatus {
    Upcoming,
    Assigned,
    Overdue,
    Completed,
    Missed,
}

#[derive(Serialize, Debug, Clone)]
struct ApiChore {
    title: String,
    description: String,
    expected_completion_time: i32,
    status: ApiStatus,
}

#[derive(Serialize, Debug, Clone)]
struct ListChoresResponse {
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
            STRFTIME('%s', 'now', 'localtime') < `expected_completion_time` AS `upcoming`,
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

        let upcoming = match row.try_get::<i32, &str>("upcoming") {
            Ok(upcoming) => upcoming == 1,
            Err(_) => {
                tracing::warn!("No upcoming information found for chore \"{}\"", title);
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

        let status = match (status, upcoming, overdue) {
            (Status::Assigned, false, false) => ApiStatus::Assigned,
            (Status::Assigned, true, false) => ApiStatus::Upcoming,
            (Status::Assigned, false, true) => ApiStatus::Overdue,
            (Status::Assigned, true, true) => {
                tracing::warn!(
                    "Chore \"{}\" is both upcoming and overdue, which should be impossible",
                    title
                );
                continue;
            }
            (Status::Completed, _, _) => ApiStatus::Completed,
            (Status::Missed, _, _) => ApiStatus::Missed,
        };

        return_chores.push(ApiChore {
            title: title,
            description,
            expected_completion_time,
            status,
        });
    }

    Ok(return_chores)
}

async fn list_chores(
    Query(params): Query<ListChoresParams>,
    Extension(pool): Extension<Arc<SqlitePool>>,
    Extension(config): Extension<Arc<Config>>,
) -> Json<ListChoresResponse> {
    match list_chores_impl(params, pool, config).await {
        Ok(chores) => Json(ListChoresResponse {
            success: true,
            chores,
            error: None,
        }),
        Err(e) => Json(ListChoresResponse {
            success: false,
            chores: Vec::new(),
            error: Some(format!("failed to fetch chores: {}", e)),
        }),
    }
}

#[derive(Deserialize, Debug)]
struct CompleteChoreParams {
    title: String,
    expected_completion_time: i32,
}

#[derive(Serialize, Debug)]
struct CompleteChoreResponse {
    success: bool,
    error: Option<String>,
}

async fn complete_chore_impl(
    params: CompleteChoreParams,
    pool: Arc<SqlitePool>,
    _config: Arc<Config>,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE `chores`
        SET
            `status` = 'completed'
        WHERE
            `title` = ?1
            AND `expected_completion_time` = ?2
        "#,
        params.title,
        params.expected_completion_time,
    )
    .execute(&*pool)
    .await?;

    Ok(())
}

async fn complete_chore(
    Form(params): Form<CompleteChoreParams>,
    Extension(pool): Extension<Arc<SqlitePool>>,
    Extension(config): Extension<Arc<Config>>,
) -> Json<CompleteChoreResponse> {
    match complete_chore_impl(params, pool, config).await {
        Ok(()) => Json(CompleteChoreResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(CompleteChoreResponse {
            success: false,
            error: Some(format!("failed to mark chore as completed: {}", e)),
        }),
    }
}

async fn index() -> impl IntoResponse {
    let mime_type = mime_guess::from_path(INDEX_PATH).first_or_text_plain();

    match read_to_string(INDEX_PATH) {
        Ok(contents) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(contents)))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body::boxed(Full::from(format!(
                "Error fetching path: {}",
                e
            ))))
            .unwrap(),
    }
}

async fn serve(pool: Arc<SqlitePool>, config: Arc<Config>) -> Result<()> {
    let serve_dir = get_service(ServeDir::new("dist")).handle_error(handle_error);

    let app = Router::new()
        .route("/", get(index))
        .nest("/dist", serve_dir.clone())
        .route("/api/chores", get(list_chores))
        .route("/api/chores/complete", post(complete_chore))
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
