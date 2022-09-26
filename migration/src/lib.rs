pub use sea_orm_migration::prelude::*;

mod m20220926_194618_create_table_yakudo_scores;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m20220926_194618_create_table_yakudo_scores::Migration,
        )]
    }
}
