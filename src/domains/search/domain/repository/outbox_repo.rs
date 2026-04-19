//! `OutboxRepository` trait for search sync event operations.

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection};

/// Represents a pending outbox event claimed by a worker.
#[derive(Clone, Debug)]
pub struct OutboxEvent {
    /// Auto-incrementing event ID.
    pub id: i64,
    /// Entity type: "record", "idol", "director", etc.
    pub entity_type: String,
    /// Primary key of the changed entity.
    pub entity_id: String,
    /// Event action: "upsert" or "delete".
    pub event_type: String,
    /// Monotonically increasing version for staleness detection.
    pub entity_version: i64,
    /// Optional JSON payload (e.g. `{"name": "..."}` for named entities).
    pub payload: Option<serde_json::Value>,
    /// Optional JSON array of record IDs affected by a named-entity change.
    pub affected_record_ids: Option<serde_json::Value>,
}

/// Repository trait for outbox table operations.
///
/// Methods accept generic `ConnectionTrait` so they work both with
/// `DatabaseConnection` (standalone) and `DatabaseTransaction` (within a txn).
#[async_trait]
pub trait OutboxRepository: Send + Sync {
    /// Insert a new sync event into the outbox table.
    async fn insert_event<C: ConnectionTrait + Send>(
        db: &C,
        entity_type: &str,
        entity_id: &str,
        event_type: &str,
        entity_version: i64,
        payload: Option<serde_json::Value>,
        affected_record_ids: Option<serde_json::Value>,
    ) -> Result<(), sea_orm::DbErr>;

    /// Claim pending events using FOR UPDATE SKIP LOCKED.
    /// Returns up to `limit` unclaimed events, marking them as claimed by `worker_id`.
    async fn claim_pending(
        db: &DatabaseConnection,
        worker_id: &str,
        limit: i64,
        lease_timeout_secs: i64,
    ) -> Result<Vec<OutboxEvent>, sea_orm::DbErr>;

    /// Mark an event as processed.
    async fn mark_processed(db: &DatabaseConnection, event_id: i64) -> Result<(), sea_orm::DbErr>;

    /// Release a claim on an event (clear `claimed_by` and `claimed_at`).
    async fn release_claim(db: &DatabaseConnection, event_id: i64) -> Result<(), sea_orm::DbErr>;

    /// Reclaim events whose claim has expired (lease timeout).
    async fn reclaim_expired_claims(
        db: &DatabaseConnection,
        lease_timeout_secs: i64,
    ) -> Result<u64, sea_orm::DbErr>;

    /// Count unprocessed events.
    async fn count_pending(db: &DatabaseConnection) -> Result<i64, sea_orm::DbErr>;
}
