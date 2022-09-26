use std::time::Duration;

use anyhow::Context;
use async_once_cell::OnceCell;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tokio::time::sleep;

static DB: OnceCell<DatabaseConnection> = OnceCell::new();

pub async fn get_db() -> anyhow::Result<&'static DatabaseConnection> {
    DB.get_or_try_init(async {
        for i in 0..10 {
            trace!("connecting to database...(try {}/10)", i + 1);

            match Database::connect(&std::env::var("DATABASE_URL")?).await {
                Ok(db) => {
                    trace!("connected to database");
                    trace!("running database migrations...");
                    Migrator::up(&db, None)
                        .await
                        .context("failed to migrate database")?;
                    trace!("database migrations completed");
                    return Ok(db);
                }
                Err(e) => {
                    warn!("failed to connect to database: {}", e);
                    warn!("retrying in 5 seconds...");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }

        Err(anyhow::anyhow!("failed to connect to database"))
    })
    .await
}
