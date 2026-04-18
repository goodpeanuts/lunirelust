//! `SearchSyncEvents` entity
//!
//! Outbox table for durable search index sync events.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as SearchSyncEventsEntity;
pub use Model as SearchSyncEventsModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "search_sync_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// Auto-incrementing event ID.
    pub id: i64,
    /// Entity type label: "record", "idol", "director", "genre", "label", "studio", "series".
    pub entity_type: String,
    /// Primary key of the entity that changed (stringified i64 for named entities, original string for records).
    pub entity_id: String,
    /// Event action: "upsert" or "delete".
    pub event_type: String,
    /// Monotonically increasing nanosecond timestamp for staleness detection.
    /// Fan-out reindex events use 0 (best-effort, never advance the tombstone).
    pub entity_version: i64,
    /// Optional JSON payload carrying entity data (e.g. `{"name": "..."}` for named entities).
    pub payload: Option<Json>,
    /// Optional JSON array of record IDs affected by a named-entity change, used for fan-out reindexing.
    pub affected_record_ids: Option<Json>,
    /// Timestamp when the event was inserted.
    pub created_at: DateTimeWithTimeZone,
    /// Timestamp when the event was successfully consumed; `NULL` means still pending.
    pub processed_at: Option<DateTimeWithTimeZone>,
    /// Worker identifier that currently holds the lease on this event; `NULL` when unclaimed.
    pub claimed_by: Option<String>,
    /// Timestamp when the lease was acquired; used to detect expired claims.
    pub claimed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
