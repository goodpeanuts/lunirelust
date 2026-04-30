//! Crawl page result entity
//!
//! Stores per-page results for auto crawl tasks.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as CrawlPageResultEntity;
pub use Model as CrawlPageResultModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crawl_page_result")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub task_id: i64,
    pub page_number: i32,
    pub status: String,
    pub records_found: i32,
    pub records_crawled: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::crawl_task::Entity",
        from = "Column::TaskId",
        to = "super::crawl_task::Column::Id"
    )]
    CrawlTask,
}

impl Related<super::crawl_task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CrawlTask.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
