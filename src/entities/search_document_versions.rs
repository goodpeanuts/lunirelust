//! `SearchDocumentVersions` entity
//!
//! Tombstone table for version tracking and delete-safe staleness detection.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as SearchDocumentVersionsEntity;
pub use Model as SearchDocumentVersionsModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "search_document_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// Entity type label (same values as `search_sync_events.entity_type`).
    pub entity_type: String,
    #[sea_orm(primary_key)]
    /// Entity primary key (stringified).
    pub entity_id: String,
    /// Highest version seen so far; stale events with a lower version are skipped.
    pub last_version: i64,
    /// `true` after a delete event has been processed for this entity.
    pub is_deleted: bool,
    /// Timestamp of the last version upsert.
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
