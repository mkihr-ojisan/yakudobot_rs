use anyhow::Context;
use async_once_cell::OnceCell;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

static DB: OnceCell<DatabaseConnection> = OnceCell::new();

pub async fn get_db() -> anyhow::Result<&'static DatabaseConnection> {
    DB.get_or_try_init(async {
        trace!("connecting to database...");
        let db =
            Database::connect(&std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?)
                .await
                .context("failed to connect to database")?;
        trace!("connected to database");
        trace!("running database migrations...");
        Migrator::up(&db, None)
            .await
            .context("failed to migrate database")?;
        trace!("database migrations completed");
        Ok(db)
    })
    .await
}
