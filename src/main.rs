use std::fs::read_to_string;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::Result;
use axum::body::Empty;
use axum::body::Full;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::get_service;
use axum::Extension;
use axum::Router;
use chrono::Duration;
use chrono::Local;
use clap::Parser;
use cron::Schedule;
use serde::Deserialize;
use sqlx::Acquire;
use sqlx::SqlitePool;
use tokio::try_join;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Deserialize, Debug)]
struct Chore {
    title: String,
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
    chores: Vec<Chore>,
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
    /// Port to bind to
    #[arg(short, long, default_value_t = 4040)]
    port: u16,

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

        for chore in config.chores.iter() {
            let chore_title = chore.title.clone();

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

async fn static_path(route: &str) -> impl IntoResponse {
    todo!();
    /*
    let path = match SIMPLE_PATHS.get(route) {
        Some(path) => path,
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(body::boxed(Empty::new()))
                .unwrap();
        }
    };

    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match read_to_string(path) {
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
    */
}

async fn handle_error(_err: std::io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

async fn serve(pool: Arc<SqlitePool>, config: Arc<Config>) -> Result<()> {
    let serve_dir = get_service(ServeDir::new("dist")).handle_error(handle_error);

    let app = Router::new()
        .route("/", get(|| async { "Hi from /" }))
        .nest("/dist", serve_dir.clone())
        /*
        .route(
            "/css/foundation.min.css",
            get(|| async { static_path("/css/foundation.min.css").await }),
        )
        .route("/for_name", get(for_name))
        .route("/weights", get(weights))
        .route("/weights_pretty", get(weights_pretty))
        */
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
