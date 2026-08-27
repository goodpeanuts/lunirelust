//! `PostgreSQL` implementation of the `OutboxRepository` trait.

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, ConnectionTrait as _, DatabaseBackend,
    DatabaseConnection, EntityTrait as _, FromQueryResult, PaginatorTrait as _, QueryFilter as _,
    Set, Statement,
};
use serde_json;

use crate::domains::search::domain::repository::outbox_repo::{OutboxEvent, OutboxRepository};
use crate::entities::search_sync_events;

/// PostgreSQL-backed implementation of `OutboxRepository`.
/// Uses raw SQL for `FOR UPDATE SKIP LOCKED` claim logic.
pub struct OutboxRepo;

/// Raw database row mapping for outbox query results.
#[derive(Debug, FromQueryResult)]
struct OutboxRow {
    /// Event ID.
    id: i64,
    /// Entity type (e.g. "record", "idol").
    entity_type: String,
    /// Entity primary key.
    entity_id: String,
    /// Event action ("upsert" or "delete").
    event_type: String,
    /// Monotonically increasing version.
    entity_version: i64,
    /// Optional JSON payload with entity data.
    payload: Option<serde_json::Value>,
    /// Optional JSON array of affected record IDs.
    affected_record_ids: Option<serde_json::Value>,
}

#[async_trait]
impl OutboxRepository for OutboxRepo {
    async fn insert_event<C: sea_orm::ConnectionTrait + Send>(
        db: &C,
        entity_type: &str,
        entity_id: &str,
        event_type: &str,
        entity_version: i64,
        payload: Option<serde_json::Value>,
        affected_record_ids: Option<serde_json::Value>,
    ) -> Result<(), sea_orm::DbErr> {
        let model = search_sync_events::ActiveModel {
            entity_type: Set(entity_type.to_owned()),
            entity_id: Set(entity_id.to_owned()),
            event_type: Set(event_type.to_owned()),
            entity_version: Set(entity_version),
            payload: Set(payload),
            affected_record_ids: Set(affected_record_ids),
            ..Default::default()
        };
        model.insert(db).await?;
        Ok(())
    }

    async fn claim_pending(
        db: &DatabaseConnection,
        worker_id: &str,
        limit: i64,
        lease_timeout_secs: i64,
    ) -> Result<Vec<OutboxEvent>, sea_orm::DbErr> {
        let sql = r#"
            UPDATE search_sync_events
            SET claimed_by = $1, claimed_at = NOW()
            WHERE id IN (
                SELECT id FROM search_sync_events
                WHERE processed_at IS NULL
                AND (
                    claimed_by IS NULL
                    OR claimed_at < NOW() - INTERVAL '1 second' * $2
                )
                ORDER BY id ASC
                LIMIT $3
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, entity_type, entity_id, event_type, entity_version, payload, affected_record_ids
            "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [worker_id.into(), lease_timeout_secs.into(), limit.into()],
        );

        let rows = OutboxRow::find_by_statement(stmt).all(db).await?;

        Ok(rows
            .into_iter()
            .map(|r| OutboxEvent {
                id: r.id,
                entity_type: r.entity_type,
                entity_id: r.entity_id,
                event_type: r.event_type,
                entity_version: r.entity_version,
                payload: r.payload,
                affected_record_ids: r.affected_record_ids,
            })
            .collect())
    }

    async fn mark_processed_batch(
        db: &DatabaseConnection,
        event_ids: &[i64],
    ) -> Result<u64, sea_orm::DbErr> {
        if event_ids.is_empty() {
            return Ok(0);
        }

        let placeholders = (1..=event_ids.len())
            .map(|index| format!("{}{}", '$', index))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "UPDATE search_sync_events
             SET processed_at = NOW(), claimed_by = NULL, claimed_at = NULL
             WHERE id IN ({placeholders})"
        );
        let values: Vec<sea_orm::Value> = event_ids.iter().copied().map(Into::into).collect();
        let result = db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                values,
            ))
            .await?;
        Ok(result.rows_affected())
    }

    async fn mark_stale_upserts_processed(db: &DatabaseConnection) -> Result<u64, sea_orm::DbErr> {
        let sql = r#"
            UPDATE search_sync_events AS event
            SET processed_at = NOW(), claimed_by = NULL, claimed_at = NULL
            FROM search_document_versions AS version
            WHERE event.processed_at IS NULL
              AND event.event_type = 'upsert'
              AND event.entity_version > 0
              AND version.entity_type = event.entity_type
              AND version.entity_id = event.entity_id
              AND (
                  version.is_deleted = TRUE
                  OR event.entity_version < version.last_version
              )
            "#;
        let result = db
            .execute(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await?;
        Ok(result.rows_affected())
    }

    async fn release_claim(db: &DatabaseConnection, event_id: i64) -> Result<(), sea_orm::DbErr> {
        let entity = search_sync_events::Entity::find_by_id(event_id)
            .one(db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::Custom(format!("Event {event_id} not found")))?;

        let mut active: search_sync_events::ActiveModel = entity.into();
        active.claimed_by = Set(None);
        active.claimed_at = Set(None);
        active.update(db).await?;
        Ok(())
    }

    async fn reclaim_expired_claims(
        db: &DatabaseConnection,
        lease_timeout_secs: i64,
    ) -> Result<u64, sea_orm::DbErr> {
        let sql = r#"
            UPDATE search_sync_events
            SET claimed_by = NULL, claimed_at = NULL
            WHERE processed_at IS NULL
            AND claimed_by IS NOT NULL
            AND claimed_at < NOW() - INTERVAL '1 second' * $1
            "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [lease_timeout_secs.into()],
        );

        let result = db.execute(stmt).await?;

        Ok(result.rows_affected())
    }

    async fn count_pending(db: &DatabaseConnection) -> Result<i64, sea_orm::DbErr> {
        let count = search_sync_events::Entity::find()
            .filter(search_sync_events::Column::ProcessedAt.is_null())
            .count(db)
            .await?;
        Ok(count as i64)
    }
}
