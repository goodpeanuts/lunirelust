//! `PostgreSQL` implementation of the `TombstoneRepository` trait.
//!
//! Uses atomic SQL (INSERT ... ON CONFLICT DO UPDATE with GREATEST) to ensure
//! version numbers only move forward, even under concurrent writes.

use async_trait::async_trait;
use chrono::{FixedOffset, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait as _, Statement};

use crate::domains::search::domain::repository::tombstone_repo::{
    DocumentVersion, TombstoneRepository,
};
use crate::entities::search_document_versions;

fn now_with_tz() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).expect("valid UTC+0 offset"))
}

/// PostgreSQL-backed implementation of `TombstoneRepository`.
///
/// Uses atomic `INSERT ... ON CONFLICT DO UPDATE` with `GREATEST()` to ensure
/// version numbers only move forward, even under concurrent writes.
pub struct TombstoneRepo;

#[async_trait]
impl TombstoneRepository for TombstoneRepo {
    async fn upsert_version<C: ConnectionTrait + Send>(
        db: &C,
        entity_type: &str,
        entity_id: &str,
        version: i64,
    ) -> Result<(), sea_orm::DbErr> {
        let now = now_with_tz();
        // Atomic upsert: only bump version forward via GREATEST.
        // This prevents concurrent transactions from regressing the version.
        let sql = r#"
            INSERT INTO search_document_versions (entity_type, entity_id, last_version, is_deleted, updated_at)
            VALUES ($1, $2, $3, FALSE, $4)
            ON CONFLICT (entity_type, entity_id) DO UPDATE SET
                last_version = GREATEST(search_document_versions.last_version, EXCLUDED.last_version),
                is_deleted = FALSE,
                updated_at = EXCLUDED.updated_at
        "#;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            [
                entity_type.into(),
                entity_id.into(),
                version.into(),
                now.into(),
            ],
        ))
        .await?;
        Ok(())
    }

    async fn upsert_versions_batch<C: ConnectionTrait + Send>(
        db: &C,
        versions: &[(String, String, i64)],
    ) -> Result<(), sea_orm::DbErr> {
        if versions.is_empty() {
            return Ok(());
        }

        let now = now_with_tz();
        let mut placeholders = Vec::with_capacity(versions.len());
        let mut values: Vec<sea_orm::Value> = Vec::with_capacity(versions.len() * 4);
        for (index, (entity_type, entity_id, version)) in versions.iter().enumerate() {
            let base = index * 4;
            placeholders.push(format!(
                "({}{}, {}{}, {}{}, FALSE, {}{})",
                '$',
                base + 1,
                '$',
                base + 2,
                '$',
                base + 3,
                '$',
                base + 4
            ));
            values.push(entity_type.clone().into());
            values.push(entity_id.clone().into());
            values.push((*version).into());
            values.push(now.into());
        }

        let sql = format!(
            "INSERT INTO search_document_versions
                 (entity_type, entity_id, last_version, is_deleted, updated_at)
             VALUES {}
             ON CONFLICT (entity_type, entity_id) DO UPDATE SET
                 last_version = GREATEST(
                     search_document_versions.last_version,
                     EXCLUDED.last_version
                 ),
                 is_deleted = CASE
                     WHEN EXCLUDED.last_version >= search_document_versions.last_version
                     THEN FALSE
                     ELSE search_document_versions.is_deleted
                 END,
                 updated_at = CASE
                     WHEN EXCLUDED.last_version >= search_document_versions.last_version
                     THEN EXCLUDED.updated_at
                     ELSE search_document_versions.updated_at
                 END",
            placeholders.join(", ")
        );
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            values,
        ))
        .await?;
        Ok(())
    }

    async fn get_version(
        db: &DatabaseConnection,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<DocumentVersion>, sea_orm::DbErr> {
        let result = search_document_versions::Entity::find_by_id((
            entity_type.to_owned(),
            entity_id.to_owned(),
        ))
        .one(db)
        .await?;

        Ok(result.map(|m| DocumentVersion {
            entity_type: m.entity_type,
            entity_id: m.entity_id,
            last_version: m.last_version,
            is_deleted: m.is_deleted,
        }))
    }

    async fn mark_deleted<C: ConnectionTrait + Send>(
        db: &C,
        entity_type: &str,
        entity_id: &str,
        version: i64,
    ) -> Result<(), sea_orm::DbErr> {
        let now = now_with_tz();
        // Atomic upsert: only bump version forward via GREATEST.
        let sql = r#"
            INSERT INTO search_document_versions (entity_type, entity_id, last_version, is_deleted, updated_at)
            VALUES ($1, $2, $3, TRUE, $4)
            ON CONFLICT (entity_type, entity_id) DO UPDATE SET
                last_version = GREATEST(search_document_versions.last_version, EXCLUDED.last_version),
                is_deleted = TRUE,
                updated_at = EXCLUDED.updated_at
        "#;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            [
                entity_type.into(),
                entity_id.into(),
                version.into(),
                now.into(),
            ],
        ))
        .await?;
        Ok(())
    }
}
