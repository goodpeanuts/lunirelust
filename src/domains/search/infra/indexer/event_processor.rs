//! Batched outbox event processing for search index updates.

use std::collections::HashMap;
use std::str::FromStr as _;
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::domains::search::domain::model::search_document::SearchDocument;
use crate::domains::search::domain::repository::outbox_repo::OutboxEvent;
use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::domain::repository::tombstone_repo::{
    DocumentVersion, TombstoneRepository as _,
};
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::infra::tombstone_repo_impl::TombstoneRepo;
use crate::domains::search::SearchEntityType;

use super::document_builder::build_record_documents;
use super::indexer_service::wrap_vectors;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Result of processing one claimed outbox batch.
pub(super) struct BatchProcessOutcome {
    pub(super) processed_ids: Vec<i64>,
    pub(super) failures: Vec<(i64, String)>,
}

#[derive(Clone)]
struct UpsertGroup {
    event: OutboxEvent,
    event_ids: Vec<i64>,
}

/// Process a claimed batch. Upserts share one search-engine task while deletes
/// preserve the existing single-document semantics.
pub(super) async fn process_event_batch(
    db: &DatabaseConnection,
    events: &[OutboxEvent],
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> BatchProcessOutcome {
    let mut outcome = BatchProcessOutcome {
        processed_ids: Vec::with_capacity(events.len()),
        failures: Vec::new(),
    };

    let upserts: Vec<OutboxEvent> = events
        .iter()
        .filter(|event| event.event_type == "upsert")
        .cloned()
        .collect();
    if !upserts.is_empty() {
        match process_upsert_events(db, &upserts, search_repo, embedding_service).await {
            Ok(ids) => outcome.processed_ids.extend(ids),
            Err(error) => {
                let message = error.to_string();
                outcome
                    .failures
                    .extend(upserts.iter().map(|event| (event.id, message.clone())));
            }
        }
    }

    for event in events.iter().filter(|event| event.event_type != "upsert") {
        match process_event(db, event, search_repo, embedding_service).await {
            Ok(()) => outcome.processed_ids.push(event.id),
            Err(error) => outcome.failures.push((event.id, error.to_string())),
        }
    }

    outcome
}

/// Process one event. This remains the ordered path for deletes and unknown
/// event types; single upserts are routed through the same batch implementation.
async fn process_event(
    db: &DatabaseConnection,
    event: &OutboxEvent,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> Result<(), BoxError> {
    match event.event_type.as_str() {
        "upsert" => {
            process_upsert_events(
                db,
                std::slice::from_ref(event),
                search_repo,
                embedding_service,
            )
            .await?;
            Ok(())
        }
        "delete" => process_delete_event(db, event, search_repo).await,
        _ => {
            tracing::warn!(event_type = %event.event_type, "Unknown event type");
            Ok(())
        }
    }
}

async fn process_upsert_events(
    db: &DatabaseConnection,
    events: &[OutboxEvent],
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> Result<Vec<i64>, BoxError> {
    let start = std::time::Instant::now();
    let mut already_processed = Vec::new();
    let mut fresh_events = Vec::new();

    for event in events {
        let version = TombstoneRepo::get_version(db, &event.entity_type, &event.entity_id).await?;
        if version
            .as_ref()
            .is_some_and(|current| is_stale_upsert(event.entity_version, current))
        {
            already_processed.push(event.id);
        } else {
            fresh_events.push(event.clone());
        }
    }

    let groups = deduplicate_upserts(&fresh_events);
    if groups.is_empty() {
        return Ok(already_processed);
    }

    let mut docs = Vec::with_capacity(groups.len());
    let record_inputs: Vec<(String, i64)> = groups
        .iter()
        .filter(|group| group.event.entity_type == SearchEntityType::Record.as_str())
        .map(|group| (group.event.entity_id.clone(), group.event.entity_version))
        .collect();

    for group in groups
        .iter()
        .filter(|group| group.event.entity_type != SearchEntityType::Record.as_str())
    {
        docs.push(construct_named_document(&group.event)?);
    }

    let record_docs = build_record_documents(db, &record_inputs).await?;
    if record_docs.len() != record_inputs.len() {
        return Err(format!(
            "Could not construct all record documents: requested={}, found={}",
            record_inputs.len(),
            record_docs.len()
        )
        .into());
    }
    docs.extend(record_docs);

    if embedding_service.is_available() {
        let record_positions: Vec<usize> = docs
            .iter()
            .enumerate()
            .filter_map(|(index, doc)| {
                (doc.entity_type == SearchEntityType::Record).then_some(index)
            })
            .collect();
        let titles: Vec<String> = record_positions
            .iter()
            .map(|index| docs[*index].title.clone())
            .collect();
        let embeddings = embedding_service.embed_batch(&titles).await;
        for (index, embedding) in record_positions.into_iter().zip(embeddings) {
            docs[index].vectors = wrap_vectors(embedding);
        }
    }

    search_repo.batch_upsert(&docs).await?;

    let versions: Vec<(String, String, i64)> = groups
        .iter()
        .filter(|group| group.event.entity_version > 0)
        .map(|group| {
            (
                group.event.entity_type.clone(),
                group.event.entity_id.clone(),
                group.event.entity_version,
            )
        })
        .collect();
    TombstoneRepo::upsert_versions_batch(db, &versions).await?;

    already_processed.extend(
        groups
            .iter()
            .flat_map(|group| group.event_ids.iter().copied()),
    );
    tracing::info!(
        events = events.len(),
        documents = docs.len(),
        elapsed_ms = start.elapsed().as_millis() as u64,
        "Outbox upsert batch indexed"
    );
    Ok(already_processed)
}

fn deduplicate_upserts(events: &[OutboxEvent]) -> Vec<UpsertGroup> {
    let mut group_indexes: HashMap<(String, String), usize> = HashMap::new();
    let mut groups: Vec<UpsertGroup> = Vec::new();

    for event in events {
        let key = (event.entity_type.clone(), event.entity_id.clone());
        if let Some(index) = group_indexes.get(&key).copied() {
            let group = &mut groups[index];
            group.event_ids.push(event.id);
            if event.entity_version >= group.event.entity_version {
                group.event = event.clone();
            }
        } else {
            group_indexes.insert(key, groups.len());
            groups.push(UpsertGroup {
                event: event.clone(),
                event_ids: vec![event.id],
            });
        }
    }

    groups
}

fn is_stale_upsert(event_version: i64, current: &DocumentVersion) -> bool {
    event_version > 0 && (current.is_deleted || event_version < current.last_version)
}

fn construct_named_document(event: &OutboxEvent) -> Result<SearchDocument, BoxError> {
    let entity_type = SearchEntityType::from_str(&event.entity_type)
        .map_err(|error: String| -> BoxError { error.into() })?;
    if entity_type == SearchEntityType::Record {
        return Err("record event passed to named document builder".into());
    }

    let title = event
        .payload
        .as_ref()
        .and_then(|payload| payload.get("name"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();

    Ok(SearchDocument {
        doc_id: format!("{}__{}", event.entity_type, event.entity_id),
        title,
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
    })
}

async fn process_delete_event(
    db: &DatabaseConnection,
    event: &OutboxEvent,
    search_repo: &Arc<MeiliSearchRepo>,
) -> Result<(), BoxError> {
    if let Some(version) =
        TombstoneRepo::get_version(db, &event.entity_type, &event.entity_id).await?
    {
        if event.entity_version > 0 && event.entity_version < version.last_version {
            return Ok(());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: i64, entity_id: &str, version: i64) -> OutboxEvent {
        OutboxEvent {
            id,
            entity_type: "record".to_owned(),
            entity_id: entity_id.to_owned(),
            event_type: "upsert".to_owned(),
            entity_version: version,
            payload: None,
            affected_record_ids: None,
        }
    }

    #[test]
    fn deduplicate_keeps_latest_version_and_all_event_ids() {
        let groups = deduplicate_upserts(&[
            event(1, "A", 1),
            event(2, "B", 1),
            event(3, "A", 3),
            event(4, "A", 2),
        ]);

        assert_eq!(groups.len(), 2);
        let group = groups
            .iter()
            .find(|group| group.event.entity_id == "A")
            .expect("A group should exist");
        assert_eq!(group.event.entity_version, 3);
        assert_eq!(group.event_ids, vec![1, 3, 4]);
    }

    #[test]
    fn stale_check_preserves_fanout_and_equal_versions() {
        let current = DocumentVersion {
            entity_type: "record".to_owned(),
            entity_id: "A".to_owned(),
            last_version: 10,
            is_deleted: false,
        };

        assert!(!is_stale_upsert(0, &current));
        assert!(is_stale_upsert(9, &current));
        assert!(!is_stale_upsert(10, &current));
        assert!(!is_stale_upsert(11, &current));
    }

    #[test]
    fn deleted_tombstone_only_prunes_real_versions() {
        let current = DocumentVersion {
            entity_type: "record".to_owned(),
            entity_id: "A".to_owned(),
            last_version: 10,
            is_deleted: true,
        };

        assert!(!is_stale_upsert(0, &current));
        assert!(is_stale_upsert(10, &current));
        assert!(is_stale_upsert(11, &current));
    }
}
