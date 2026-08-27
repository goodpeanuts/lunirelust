//! Full PostgreSQL-to-MeiliSearch sync using bounded document batches.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::domain::repository::tombstone_repo::TombstoneRepository as _;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::infra::tombstone_repo_impl::TombstoneRepo;
use crate::domains::search::SearchEntityType;

use super::document_builder::{build_documents_for_ids, fetch_entity_ids};
use super::indexer_service::wrap_vectors;

pub(super) const SYNC_BATCH_SIZE: usize = 100;

/// Populate an empty index from the primary database.
///
/// Every entity type uses the same bounded batch path, so a full recovery
/// creates at most one search-engine task per 100 documents.
pub(super) async fn run_full_sync(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sync_version = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000);
    let full_sync_start = std::time::Instant::now();

    for &entity_type in SearchEntityType::ALL {
        let entity_start = std::time::Instant::now();
        let ids = fetch_entity_ids(db, entity_type).await?;
        let total = ids.len();
        let mut indexed = 0usize;

        for chunk in ids.chunks(SYNC_BATCH_SIZE) {
            let mut docs = build_documents_for_ids(db, entity_type, chunk, sync_version).await?;
            add_embeddings(&mut docs, embedding_service).await;
            search_repo.batch_upsert(&docs).await?;

            let versions: Vec<(String, String, i64)> = docs
                .iter()
                .map(|doc| {
                    (
                        doc.entity_type.as_str().to_owned(),
                        doc.entity_id.clone(),
                        sync_version,
                    )
                })
                .collect();
            TombstoneRepo::upsert_versions_batch(db, &versions).await?;

            indexed += docs.len();
            let batch_number = indexed.div_ceil(SYNC_BATCH_SIZE);
            if batch_number.is_multiple_of(5) || indexed == total {
                tracing::info!(
                    entity_type = entity_type.as_str(),
                    indexed,
                    total,
                    elapsed_ms = entity_start.elapsed().as_millis() as u64,
                    "Full sync progress"
                );
            }
        }

        tracing::info!(
            entity_type = entity_type.as_str(),
            indexed,
            total,
            elapsed_ms = entity_start.elapsed().as_millis() as u64,
            "Full sync entity complete"
        );
    }

    tracing::info!(
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync complete"
    );
    Ok(())
}

pub(super) async fn add_embeddings(
    docs: &mut [crate::domains::search::domain::model::search_document::SearchDocument],
    embedding_service: &Arc<EmbeddingService>,
) {
    if !embedding_service.is_available() {
        return;
    }

    let record_positions: Vec<usize> = docs
        .iter()
        .enumerate()
        .filter_map(|(index, doc)| (doc.entity_type == SearchEntityType::Record).then_some(index))
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
