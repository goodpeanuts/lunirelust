//! Crawl task entity
//!
//! Stores crawl task metadata, status, and serialized input payload.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as CrawlTaskEntity;
pub use Model as CrawlTaskModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crawl_task")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub task_type: String,
    pub status: String,
    pub user_id: String,
    pub mark_liked: bool,
    pub mark_viewed: bool,
    pub input_payload: Option<String>,
    pub max_pages: Option<i32>,
    pub total_codes: i32,
    pub success_count: i32,
    pub fail_count: i32,
    pub skip_count: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::crawl_code_result::Entity")]
    CodeResults,
    #[sea_orm(has_many = "super::crawl_page_result::Entity")]
    PageResults,
}

impl Related<super::crawl_code_result::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CodeResults.def()
    }
}

impl Related<super::crawl_page_result::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PageResults.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
