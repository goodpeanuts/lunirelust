//! Crawl entity progress entity
//!
//! Round-only per-entity crawl state for the entity-auto-crawl task type.
//! Stores `last_crawled_round` (the rotation counter) plus a pointer to the
//! latest task. There is intentionally no `status` column: success or failure
//! is derived by joining `crawl_task` via `last_task_id`.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as CrawlEntityProgressEntity;
pub use Model as CrawlEntityProgressModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crawl_entity_progress")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub entity_name: String,
    pub last_crawled_round: i32,
    pub last_task_id: Option<i64>,
    pub last_crawled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
