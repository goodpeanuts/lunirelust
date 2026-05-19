//! Crawl code result entity
//!
//! Stores per-code results for crawl tasks.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as CrawlCodeResultEntity;
pub use Model as CrawlCodeResultModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crawl_code_result")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub task_id: i64,
    pub code: String,
    pub status: String,
    pub record_id: Option<String>,
    pub images_downloaded: i32,
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
