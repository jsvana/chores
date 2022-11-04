use std::fs::read_to_string;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::Result;
use chrono::Duration;
use chrono::Local;
use clap::Parser;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::Acquire;
use sqlx::SqlitePool;

#[derive(Deserialize, Debug)]
struct Chore {
    title: String,
    description: String,
    frequency: String,
}

const ONE_DAY: StdDuration = StdDuration::from_secs(86400);
const fn one_day() -> StdDuration {
    ONE_DAY
}

const ONE_HOUR: StdDuration = StdDuration::from_secs(3600);
const fn one_hour() -> StdDuration {
    ONE_HOUR
}

#[derive(Deserialize, Debug)]
struct Config {
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config = Config::from_path(&args.config_path)?;

    tracing_subscriber::fmt::init();

    let pool = Arc::new(SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?);
    sqlx::migrate!().run(&*pool).await?;

    tokio::spawn(update_chores(pool.clone(), config.clone()));

    loop {}
}
