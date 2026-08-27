//! Reconciliation of primary-database and search-engine entity ID sets.

use std::collections::HashSet;
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::domain::repository::tombstone_repo::TombstoneRepository as _;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::infra::tombstone_repo_impl::TombstoneRepo;
use crate::domains::search::SearchEntityType;

use super::document_builder::{build_documents_for_ids, fetch_entity_ids};
use super::full_sync::{add_embeddings, SYNC_BATCH_SIZE};
use super::indexer_service::wrap_vectors;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Repair only missing and ghost documents, then verify every entity ID set.
pub(super) async fn reconcile_index(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> bool {
    let start = std::time::Instant::now();
    let repair_version = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000);

    for &entity_type in SearchEntityType::ALL {
        if let Err(error) = repair_entity(
            db,
            search_repo,
            embedding_service,
            entity_type,
            repair_version,
        )
        .await
        {
            tracing::error!(
                entity_type = entity_type.as_str(),
                "Search reconciliation failed: {error}"
            );
            return false;
        }
    }

    for &entity_type in SearchEntityType::ALL {
        let pg_ids = match fetch_entity_ids(db, entity_type).await {
            Ok(ids) => ids.into_iter().collect::<HashSet<_>>(),
            Err(error) => {
                tracing::error!(
                    entity_type = entity_type.as_str(),
                    "Failed to verify PostgreSQL IDs: {error}"
                );
                return false;
            }
        };
        let meili_ids = match search_repo.get_entity_ids(entity_type).await {
            Ok(ids) => ids.into_iter().collect::<HashSet<_>>(),
            Err(error) => {
                tracing::error!(
                    entity_type = entity_type.as_str(),
                    "Failed to verify MeiliSearch IDs: {error}"
                );
                return false;
            }
        };
        if pg_ids != meili_ids {
            tracing::warn!(
                entity_type = entity_type.as_str(),
                postgres = pg_ids.len(),
                meilisearch = meili_ids.len(),
                "Search reconciliation verification mismatch"
            );
            return false;
        }
    }

    tracing::info!(
        elapsed_ms = start.elapsed().as_millis() as u64,
        "Search reconciliation complete: all entity ID sets match"
    );
    true
}

async fn repair_entity(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
    entity_type: SearchEntityType,
    repair_version: i64,
) -> Result<(), BoxError> {
    let pg_ids: HashSet<String> = fetch_entity_ids(db, entity_type)
        .await?
        .into_iter()
        .collect();
    let meili_ids: HashSet<String> = search_repo
        .get_entity_ids(entity_type)
        .await?
        .into_iter()
        .collect();

    let mut missing: Vec<String> = pg_ids.difference(&meili_ids).cloned().collect();
    let mut ghosts: Vec<String> = meili_ids.difference(&pg_ids).cloned().collect();
    missing.sort_unstable();
    ghosts.sort_unstable();

    if !missing.is_empty() || !ghosts.is_empty() {
        tracing::warn!(
            entity_type = entity_type.as_str(),
            missing = missing.len(),
            ghosts = ghosts.len(),
            "Repairing search index ID differences"
        );
    }

    for chunk in missing.chunks(SYNC_BATCH_SIZE) {
        let mut docs = build_documents_for_ids(db, entity_type, chunk, repair_version).await?;
        if docs.len() != chunk.len() {
            return Err(format!(
                "Could not construct all missing {} documents: requested={}, found={}",
                entity_type.as_str(),
                chunk.len(),
                docs.len()
            )
            .into());
        }
        add_embeddings(&mut docs, embedding_service).await;
        search_repo.batch_upsert(&docs).await?;

        let versions: Vec<(String, String, i64)> = docs
            .iter()
            .map(|doc| {
                (
                    doc.entity_type.as_str().to_owned(),
                    doc.entity_id.clone(),
                    repair_version,
                )
            })
            .collect();
        TombstoneRepo::upsert_versions_batch(db, &versions).await?;
    }

    for chunk in ghosts.chunks(SYNC_BATCH_SIZE) {
        let doc_ids: Vec<String> = chunk
            .iter()
            .map(|entity_id| format!("{}__{entity_id}", entity_type.as_str()))
            .collect();
        search_repo.batch_delete(&doc_ids).await?;
    }

    Ok(())
}

/// Backfill vector embeddings for record documents that were indexed without them.
pub(super) async fn backfill_missing_vectors(
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) {
    let batch_size = 50;
    let mut total_updated = 0usize;
    let mut offset = 0usize;
    let mut iterations = 0usize;
    const MAX_BACKFILL_ITERATIONS: usize = 200;

    loop {
        let (mut docs, raw_page_size) = match search_repo
            .find_records_missing_vectors(offset, batch_size)
            .await
        {
            Ok(pair) => pair,
            Err(error) => {
                tracing::debug!("find_records_missing_vectors failed: {error}");
                break;
            }
        };

        if raw_page_size < batch_size && docs.is_empty() {
            break;
        }
        offset += raw_page_size;
        if docs.is_empty() {
            continue;
        }

        iterations += 1;
        if iterations > MAX_BACKFILL_ITERATIONS {
            tracing::info!(
                "Backfill iteration limit reached, remaining docs will be picked up later"
            );
            break;
        }

        let titles: Vec<String> = docs.iter().map(|doc| doc.title.clone()).collect();
        let embeddings = embedding_service.embed_batch(&titles).await;
        let actual_embeddings = embeddings.iter().filter(|value| value.is_some()).count();
        for (doc, embedding) in docs.iter_mut().zip(embeddings) {
            doc.vectors = wrap_vectors(embedding);
        }

        if let Err(error) = search_repo.batch_upsert(&docs).await {
            tracing::warn!("Vector backfill batch failed: {error}");
            break;
        }
        total_updated += docs.len();

        if actual_embeddings == 0 {
            tracing::warn!("Embedding service returned no vectors, stopping backfill");
            break;
        }
        if raw_page_size < batch_size {
            break;
        }
    }

    if total_updated > 0 {
        tracing::info!(total_updated, "Vector backfill complete");
    }
}
