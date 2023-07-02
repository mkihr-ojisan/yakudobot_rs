use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(YakudoScores::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(YakudoScores::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(YakudoScores::Username).string().not_null())
                    .col(ColumnDef::new(YakudoScores::NoteId).string().not_null())
                    .col(ColumnDef::new(YakudoScores::QuoteId).string().not_null())
                    .col(ColumnDef::new(YakudoScores::Score).double().not_null())
                    .col(ColumnDef::new(YakudoScores::Date).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(YakudoScores::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum YakudoScores {
    Table,
    Id,
    Username,
    NoteId,
    QuoteId,
    Score,
    Date,
}
