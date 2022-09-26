use anyhow::Context;
use async_once_cell::OnceCell;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

static DB: OnceCell<DatabaseConnection> = OnceCell::new();

pub async fn get_db() -> anyhow::Result<&'static DatabaseConnection> {
    DB.get_or_try_init(async {
        let db =
            Database::connect(&std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?)
                .await
                .context("failed to connect to database")?;
        Migrator::up(&db, None)
            .await
            .context("failed to migrate database")?;
        Ok(db)
    })
    .await
}
