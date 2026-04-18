//! Helper functions for inserting search outbox events within Luna CUD transactions.

use crate::domains::search::{
    OutboxRepo, OutboxRepository as _, SearchEntityType, TombstoneRepo, TombstoneRepository as _,
};
use crate::entities::{idol_participation, record, record_genre};
use chrono::Utc;
use sea_orm::{
    ColumnTrait as _, ConnectionTrait, EntityTrait as _, FromQueryResult, QueryFilter as _,
};
use serde_json::json;

/// Find record IDs that reference a named entity via FK column.
async fn find_record_ids_by_fk<C: ConnectionTrait>(
    db: &C,
    column: record::Column,
    entity_id: i64,
) -> Result<Vec<String>, sea_orm::DbErr> {
    #[derive(FromQueryResult)]
    struct IdRow {
        id: String,
    }
    let rows: Vec<IdRow> = record::Entity::find()
        .filter(column.eq(entity_id))
        .into_model::<IdRow>()
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.id).collect())
}

/// Find record IDs linked to a genre via junction table.
async fn find_record_ids_by_genre<C: ConnectionTrait>(
    db: &C,
    genre_id: i64,
) -> Result<Vec<String>, sea_orm::DbErr> {
    #[derive(FromQueryResult)]
    struct IdRow {
        record_id: String,
    }
    let rows: Vec<IdRow> = record_genre::Entity::find()
        .filter(record_genre::Column::GenreId.eq(genre_id))
        .into_model::<IdRow>()
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.record_id).collect())
}

/// Find record IDs linked to an idol via junction table.
async fn find_record_ids_by_idol<C: ConnectionTrait>(
    db: &C,
    idol_id: i64,
) -> Result<Vec<String>, sea_orm::DbErr> {
    #[derive(FromQueryResult)]
    struct IdRow {
        record_id: String,
    }
    let rows: Vec<IdRow> = idol_participation::Entity::find()
        .filter(idol_participation::Column::IdolId.eq(idol_id))
        .into_model::<IdRow>()
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.record_id).collect())
}

/// Find record IDs affected by a named entity change.
/// Dispatches to the correct query based on `entity_type`.
pub async fn find_affected_record_ids<C: ConnectionTrait>(
    db: &C,
    entity_type: SearchEntityType,
    entity_id: i64,
) -> Result<Vec<String>, sea_orm::DbErr> {
    match entity_type {
        SearchEntityType::Director => {
            find_record_ids_by_fk(db, record::Column::DirectorId, entity_id).await
        }
        SearchEntityType::Studio => {
            find_record_ids_by_fk(db, record::Column::StudioId, entity_id).await
        }
        SearchEntityType::Label => {
            find_record_ids_by_fk(db, record::Column::LabelId, entity_id).await
        }
        SearchEntityType::Series => {
            find_record_ids_by_fk(db, record::Column::SeriesId, entity_id).await
        }
        SearchEntityType::Genre => find_record_ids_by_genre(db, entity_id).await,
        SearchEntityType::Idol => find_record_ids_by_idol(db, entity_id).await,
        SearchEntityType::Record => Ok(Vec::new()),
    }
}

/// Insert outbox upsert event + upsert tombstone for a named entity.
/// The `name` parameter is stored in the event payload so the indexer can reconstruct
/// the document without querying the database.
pub async fn outbox_entity_upsert<C: ConnectionTrait + Send>(
    db: &C,
    entity_type: SearchEntityType,
    entity_id: i64,
    entity_name: &str,
    affected_record_ids: Vec<String>,
) -> Result<(), sea_orm::DbErr> {
    let version = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_millis() * 1_000_000);
    let entity_id_str = entity_id.to_string();
    let affected_json = if affected_record_ids.is_empty() {
        None
    } else {
        Some(json!(affected_record_ids))
    };
    let payload = Some(json!({ "name": entity_name }));

    OutboxRepo::insert_event(
        db,
        entity_type.as_str(),
        &entity_id_str,
        "upsert",
        version,
        payload,
        affected_json,
    )
    .await?;
    TombstoneRepo::upsert_version(db, entity_type.as_str(), &entity_id_str, version).await?;
    Ok(())
}

/// Insert outbox delete event + mark tombstone for a named entity.
pub async fn outbox_entity_delete<C: ConnectionTrait + Send>(
    db: &C,
    entity_type: SearchEntityType,
    entity_id: i64,
    affected_record_ids: Vec<String>,
) -> Result<(), sea_orm::DbErr> {
    let version = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_millis() * 1_000_000);
    let entity_id_str = entity_id.to_string();
    let affected_json = if affected_record_ids.is_empty() {
        None
    } else {
        Some(json!(affected_record_ids))
    };

    OutboxRepo::insert_event(
        db,
        entity_type.as_str(),
        &entity_id_str,
        "delete",
        version,
        None,
        affected_json,
    )
    .await?;
    TombstoneRepo::mark_deleted(db, entity_type.as_str(), &entity_id_str, version).await?;
    Ok(())
}

/// Insert fan-out reindex events for affected records.
/// Called after a named entity update/delete to trigger record reindexing.
/// Fan-out events use version=0 so the indexer treats them as best-effort
/// hints: they never advance the tombstone, and a concurrent real record
/// edit with a higher version will always win.
pub async fn outbox_fanout_records<C: ConnectionTrait + Send>(
    db: &C,
    record_ids: &[String],
) -> Result<(), sea_orm::DbErr> {
    for record_id in record_ids {
        OutboxRepo::insert_event(
            db,
            SearchEntityType::Record.as_str(),
            record_id,
            "upsert",
            0,
            None,
            None,
        )
        .await?;
    }
    Ok(())
}
