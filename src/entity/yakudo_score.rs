use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "yakudo_scores")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub username: String,
    #[sea_orm(column_type = "BigInteger")]
    pub tweet_id: u64,
    #[sea_orm(column_type = "BigInteger")]
    pub retweet_id: u64,
    pub score: f64,
    pub date: chrono::DateTime<chrono::Local>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
