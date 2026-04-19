//! Outbox event processing: upsert and delete events for search index updates.

use std::str::FromStr as _;
use std::sync::Arc;

use sea_orm::{DatabaseConnection, EntityTrait as _};

use crate::domains::search::domain::model::search_document::SearchDocument;
use crate::domains::search::domain::repository::outbox_repo::OutboxEvent;
use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::domain::repository::tombstone_repo::TombstoneRepository as _;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::infra::tombstone_repo_impl::TombstoneRepo;
use crate::domains::search::SearchEntityType;

use super::indexer_service::wrap_vectors;

/// Process a single outbox event: dispatches to upsert or delete handler based on event type.
pub(super) async fn process_event(
    db: &DatabaseConnection,
    event: &OutboxEvent,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    let result = match event.event_type.as_str() {
        "upsert" => process_upsert_event(db, event, search_repo, embedding_service).await,
        "delete" => process_delete_event(db, event, search_repo).await,
        _ => {
            tracing::warn!(event_type = %event.event_type, "Unknown event type");
            return Ok(());
        }
    };
    if result.is_ok() {
        tracing::debug!(
            event_type = %event.event_type,
            entity_type = %event.entity_type,
            entity_id = %event.entity_id,
            event_id = event.id,
            elapsed_ms = start.elapsed().as_millis() as u64,
            "Event processed"
        );
    }
    result
}

/// Handle an upsert event: check tombstone staleness, build the document, generate
/// embedding if available, upsert into `MeiliSearch`, and update the tombstone version.
async fn process_upsert_event(
    db: &DatabaseConnection,
    event: &OutboxEvent,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check tombstone
    if let Some(version) =
        TombstoneRepo::get_version(db, &event.entity_type, &event.entity_id).await?
    {
        if version.is_deleted {
            return Ok(());
        }
        if event.entity_version < version.last_version && event.entity_version > 0 {
            return Ok(());
        }
    }

    // Construct and index document
    let mut doc = construct_document(db, event).await?;

    // Generate embedding for record documents when embedding service is available
    if doc.entity_type == SearchEntityType::Record && embedding_service.is_available() {
        doc.vectors = wrap_vectors(embedding_service.embed(&doc.title).await);
    }

    search_repo.upsert_document(&doc).await?;

    // Update tombstone version. Fan-out events arrive with entity_version=0;
    // writing 0 would regress the tombstone and allow stale replay. Only
    // update for events that carry a real version.
    if event.entity_version > 0 {
        TombstoneRepo::upsert_version(
            db,
            &event.entity_type,
            &event.entity_id,
            event.entity_version,
        )
        .await?;
    }

    Ok(())
}

/// Handle a delete event: verify staleness, remove the document from `MeiliSearch`,
/// and mark the tombstone as deleted.
async fn process_delete_event(
    db: &DatabaseConnection,
    event: &OutboxEvent,
    search_repo: &Arc<MeiliSearchRepo>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Reject stale delete events.
    if let Some(version) =
        TombstoneRepo::get_version(db, &event.entity_type, &event.entity_id).await?
    {
        // If a newer version exists and this event carries a real version, skip
        if event.entity_version > 0 && event.entity_version < version.last_version {
            return Ok(());
        }
        // Note: we do NOT skip when is_deleted=true because the tombstone is
        // marked within the same transaction as the outbox event. The indexer
        // must still delete the document from MeiliSearch.
    }

    let doc_id = format!("{}__{}", event.entity_type, event.entity_id);
    search_repo.delete_document(&doc_id).await?;
    TombstoneRepo::mark_deleted(
        db,
        &event.entity_type,
        &event.entity_id,
        event.entity_version,
    )
    .await?;
    Ok(())
}

/// Construct a `SearchDocument` from the outbox event.
/// For named entities, reads the title from the event payload.
/// For records, queries all related entity names from the database.
async fn construct_document(
    db: &DatabaseConnection,
    event: &OutboxEvent,
) -> Result<SearchDocument, Box<dyn std::error::Error + Send + Sync>> {
    let doc_id = format!("{}__{}", event.entity_type, event.entity_id);
    let entity_type = SearchEntityType::from_str(&event.entity_type)
        .map_err(|e: String| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;

    if entity_type != SearchEntityType::Record {
        let title = event
            .payload
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        return Ok(SearchDocument {
            doc_id,
            title: title.to_owned(),
            entity_type,
            entity_id: event.entity_id.clone(),
            entity_version: event.entity_version,
            permission: 0,
            date: None,
            duration: None,
            director_name: None,
            studio_name: None,
            label_name: None,
            series_name: None,
            genre_names: None,
            idol_names: None,
            vectors: None,
        });
    }

    use crate::entities::{
        director, genre, idol, idol_participation, label, record, record_genre, series, studio,
    };
    use sea_orm::{ColumnTrait as _, QueryFilter as _};
    let r = record::Entity::find_by_id(&event.entity_id)
        .one(db)
        .await?
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Record {} not found", event.entity_id).into()
        })?;

    // Load related named entities
    let director_name = director::Entity::find_by_id(r.director_id)
        .one(db)
        .await?
        .map(|d| d.name);
    let studio_name = studio::Entity::find_by_id(r.studio_id)
        .one(db)
        .await?
        .map(|s| s.name);
    let label_name = label::Entity::find_by_id(r.label_id)
        .one(db)
        .await?
        .map(|l| l.name);
    let series_name = series::Entity::find_by_id(r.series_id)
        .one(db)
        .await?
        .map(|s| s.name);

    // Load genre names via junction table
    let genre_names: Vec<String> = {
        let rg_rows = record_genre::Entity::find()
            .filter(record_genre::Column::RecordId.eq(&r.id))
            .find_also_related(genre::Entity)
            .all(db)
            .await?;
        rg_rows
            .into_iter()
            .filter_map(|(_, g)| g.map(|genre| genre.name))
            .collect()
    };

    // Load idol names via junction table
    let idol_names: Vec<String> = {
        let ip_rows = idol_participation::Entity::find()
            .filter(idol_participation::Column::RecordId.eq(&r.id))
            .find_also_related(idol::Entity)
            .all(db)
            .await?;
        ip_rows
            .into_iter()
            .filter_map(|(_, i)| i.map(|idol| idol.name))
            .collect()
    };

    Ok(SearchDocument {
        doc_id,
        title: r.title.clone(),
        entity_type: SearchEntityType::Record,
        entity_id: r.id.clone(),
        entity_version: event.entity_version,
        permission: r.permission,
        date: Some(r.date.to_string()),
        duration: Some(r.duration),
        director_name,
        studio_name,
        label_name,
        series_name,
        genre_names: Some(genre_names),
        idol_names: Some(idol_names),
        vectors: None,
    })
}
