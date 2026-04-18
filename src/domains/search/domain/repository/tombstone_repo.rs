//! `TombstoneRepository` trait for search document version tracking.

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection};

/// Current version status of a document.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DocumentVersion {
    pub entity_type: String,
    pub entity_id: String,
    pub last_version: i64,
    pub is_deleted: bool,
}

/// Repository trait for tombstone table operations.
///
/// Methods accept generic `ConnectionTrait` so they work both with
/// `DatabaseConnection` (standalone) and `DatabaseTransaction` (within a txn).
#[async_trait]
pub trait TombstoneRepository: Send + Sync {
    /// Upsert a version entry. If the entity already exists, update the version.
    async fn upsert_version<C: ConnectionTrait + Send>(
        db: &C,
        entity_type: &str,
        entity_id: &str,
        version: i64,
    ) -> Result<(), sea_orm::DbErr>;

    /// Get the current version status for an entity.
    async fn get_version(
        db: &DatabaseConnection,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<DocumentVersion>, sea_orm::DbErr>;

    /// Mark an entity as deleted in the tombstone table.
    async fn mark_deleted<C: ConnectionTrait + Send>(
        db: &C,
        entity_type: &str,
        entity_id: &str,
        version: i64,
    ) -> Result<(), sea_orm::DbErr>;
}
