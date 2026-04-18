//! Reconciliation: detect and repair divergence between `PostgreSQL` and `MeiliSearch`.
//!
//! Includes count comparison, ghost document removal, and vector backfill.

use std::str::FromStr as _;
use std::sync::Arc;

use sea_orm::{DatabaseConnection, EntityTrait as _, PaginatorTrait as _, QuerySelect as _};

use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::SearchEntityType;

use super::full_sync::run_full_sync;
use super::indexer_service::{ignore_result, wrap_vectors};

/// Check `PostgreSQL` vs `MeiliSearch` document counts and repair divergences.
///
/// When `MeiliSearch` has fewer documents (pg > meili), run a full sync to
/// upsert missing documents. When `MeiliSearch` has more (meili > pg), fetch
/// entity IDs from both sides and delete ghost documents from `MeiliSearch`.
pub(super) async fn reconcile_counts(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> bool {
    use crate::entities::{director, genre, idol, label, record, series, studio};

    // Reconcile records
    let pg_record_count = record::Entity::find().count(db).await.unwrap_or(0);
    let meili_record_count = search_repo
        .get_document_count(SearchEntityType::Record)
        .await
        .unwrap_or(0);

    if pg_record_count != meili_record_count {
        if meili_record_count > pg_record_count {
            // Ghost documents: delete MeiliSearch documents not in PostgreSQL
            if !remove_ghost_documents(db, search_repo, SearchEntityType::Record.as_str()).await {
                return false;
            }
        } else {
            // Missing documents: full sync to upsert
            tracing::warn!(
                "Record count mismatch: PostgreSQL={pg_record_count}, MeiliSearch={meili_record_count}. Running full sync..."
            );
            if let Err(e) = run_full_sync(db, search_repo, embedding_service).await {
                tracing::error!("Reconciliation full sync failed: {}", e);
                return false;
            }
            return true; // full_sync already handles all entity types
        }
    }

    // Reconcile named entities
    let entity_checks: Vec<(SearchEntityType, u64, u64)> = {
        let mut checks = Vec::new();
        let pg = director::Entity::find().count(db).await.unwrap_or(0);
        let meili = search_repo
            .get_document_count(SearchEntityType::Director)
            .await
            .unwrap_or(0);
        checks.push((SearchEntityType::Director, pg, meili));
        let pg = studio::Entity::find().count(db).await.unwrap_or(0);
        let meili = search_repo
            .get_document_count(SearchEntityType::Studio)
            .await
            .unwrap_or(0);
        checks.push((SearchEntityType::Studio, pg, meili));
        let pg = label::Entity::find().count(db).await.unwrap_or(0);
        let meili = search_repo
            .get_document_count(SearchEntityType::Label)
            .await
            .unwrap_or(0);
        checks.push((SearchEntityType::Label, pg, meili));
        let pg = series::Entity::find().count(db).await.unwrap_or(0);
        let meili = search_repo
            .get_document_count(SearchEntityType::Series)
            .await
            .unwrap_or(0);
        checks.push((SearchEntityType::Series, pg, meili));
        let pg = genre::Entity::find().count(db).await.unwrap_or(0);
        let meili = search_repo
            .get_document_count(SearchEntityType::Genre)
            .await
            .unwrap_or(0);
        checks.push((SearchEntityType::Genre, pg, meili));
        let pg = idol::Entity::find().count(db).await.unwrap_or(0);
        let meili = search_repo
            .get_document_count(SearchEntityType::Idol)
            .await
            .unwrap_or(0);
        checks.push((SearchEntityType::Idol, pg, meili));
        checks
    };

    for (entity_type, pg_count, meili_count) in entity_checks {
        if pg_count != meili_count {
            if meili_count > pg_count {
                if !remove_ghost_documents(db, search_repo, entity_type.as_str()).await {
                    return false;
                }
            } else {
                tracing::warn!(
                    "{entity_type} count mismatch: PostgreSQL={pg_count}, MeiliSearch={meili_count}. Running full sync..."
                );
                if let Err(e) = run_full_sync(db, search_repo, embedding_service).await {
                    tracing::error!("Reconciliation full sync failed: {}", e);
                    return false;
                }
                return true;
            }
        }
    }
    tracing::info!(
        "Reconciliation complete: all entity counts match between PostgreSQL and MeiliSearch"
    );
    true
}

/// Remove ghost documents from `MeiliSearch` that no longer exist in `PostgreSQL`.
///
/// Fetches entity IDs from both sides and deletes any `MeiliSearch` documents
/// whose entity ID is not present in `PostgreSQL`. Returns `true` if all
/// deletions succeeded, `false` if any failed.
async fn remove_ghost_documents(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    entity_type: &str,
) -> bool {
    use std::collections::HashSet;

    // Get all entity IDs from MeiliSearch for this type
    let meili_ids = match search_repo
        .get_entity_ids(
            SearchEntityType::from_str(entity_type)
                .ok()
                .unwrap_or(SearchEntityType::Record),
        )
        .await
    {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!("Failed to get MeiliSearch IDs for {entity_type}: {e}");
            return false;
        }
    };

    // Get all entity IDs from PostgreSQL for this type
    let pg_ids: HashSet<String> = {
        use crate::entities::{director, genre, idol, label, series, studio};
        match entity_type {
            "record" => fetch_pg_record_ids(db).await,
            "director" => {
                fetch_pg_i64_ids::<director::Entity, director::Column>(db, director::Column::Id)
                    .await
            }
            "studio" => {
                fetch_pg_i64_ids::<studio::Entity, studio::Column>(db, studio::Column::Id).await
            }
            "label" => {
                fetch_pg_i64_ids::<label::Entity, label::Column>(db, label::Column::Id).await
            }
            "series" => {
                fetch_pg_i64_ids::<series::Entity, series::Column>(db, series::Column::Id).await
            }
            "genre" => {
                fetch_pg_i64_ids::<genre::Entity, genre::Column>(db, genre::Column::Id).await
            }
            "idol" => fetch_pg_i64_ids::<idol::Entity, idol::Column>(db, idol::Column::Id).await,
            _ => return true,
        }
    };

    let meili_set: HashSet<String> = meili_ids.into_iter().collect();
    let ghosts: Vec<&String> = meili_set.difference(&pg_ids).collect();

    if ghosts.is_empty() {
        return true;
    }

    tracing::warn!(
        "Found {} ghost {entity_type} documents in MeiliSearch, removing...",
        ghosts.len()
    );

    let mut all_ok = true;
    for ghost_id in ghosts {
        let doc_id = format!("{entity_type}__{ghost_id}");
        if let Err(e) = search_repo.delete_document(&doc_id).await {
            tracing::warn!("Failed to delete ghost document {}: {}", doc_id, e);
            all_ok = false;
        }
    }
    if !all_ok {
        tracing::warn!("Some ghost {entity_type} documents could not be removed");
    }
    all_ok
}

/// Fetch all record IDs from `PostgreSQL` (records use String IDs).
async fn fetch_pg_record_ids(db: &DatabaseConnection) -> std::collections::HashSet<String> {
    use crate::entities::record;
    record::Entity::find()
        .all(db)
        .await
        .map(|rows| rows.iter().map(|r| r.id.clone()).collect())
        .unwrap_or_default()
}

/// Fetch all i64 PK IDs from a `PostgreSQL` entity table, converted to strings.
async fn fetch_pg_i64_ids<E, C>(
    db: &DatabaseConnection,
    col: C,
) -> std::collections::HashSet<String>
where
    E: sea_orm::EntityTrait,
    C: sea_orm::ColumnTrait,
{
    let rows = E::find()
        .select_only()
        .column(col)
        .into_tuple::<i64>()
        .all(db)
        .await
        .unwrap_or_default();

    rows.into_iter().map(|id| id.to_string()).collect()
}

/// Backfill vector embeddings for record documents that were indexed without them.
///
/// Pages through the `MeiliSearch` index using offset-based pagination, filters
/// for records missing `_vectors`, generates embeddings via vLLM, and upserts
/// the updated documents. Stops after `MAX_BACKFILL_ITERATIONS` iterations to
/// bound resource usage; remaining docs are picked up on the next reconciliation.
pub(super) async fn backfill_missing_vectors(
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) {
    let batch_size = 50;
    let mut total_updated = 0usize;
    let mut offset = 0usize;
    let mut iterations = 0usize;
    const MAX_BACKFILL_ITERATIONS: usize = 200; // ~10K docs per recovery

    loop {
        iterations += 1;
        if iterations > MAX_BACKFILL_ITERATIONS {
            tracing::info!("Backfill iteration limit reached, remaining docs will be picked up by reconciliation");
            break;
        }
        let (docs, raw_page_size) = match search_repo
            .find_records_missing_vectors(offset, batch_size)
            .await
        {
            Ok(pair) => pair,
            Err(e) => {
                tracing::debug!("find_records_missing_vectors failed: {}", e);
                break;
            }
        };

        // If MeiliSearch returned fewer docs than batch_size, we've
        // reached the end of the corpus.
        if raw_page_size < batch_size && docs.is_empty() {
            break;
        }

        // Advance offset by raw page size regardless of how many were
        // missing vectors, so we scan the full index.
        offset += raw_page_size;

        if docs.is_empty() {
            // No missing-vector docs on this page, but there may be more
            // pages. Continue scanning.
            continue;
        }

        tracing::info!(
            "Backfilling {} records missing vectors (offset {})...",
            docs.len(),
            offset
        );

        let titles: Vec<String> = docs.iter().map(|d| d.title.clone()).collect();
        let embeddings = embedding_service.embed_batch(&titles).await;

        // Count how many embeddings were actually generated (not None).
        // If all are None, the embedding service is unavailable — stop
        // looping to avoid an infinite cycle of fetching and re-upserting
        // the same documents without vectors.
        let actual_embeddings: usize = embeddings.iter().filter(|e| e.is_some()).count();

        for (mut doc, emb) in docs.into_iter().zip(embeddings.into_iter()) {
            doc.vectors = wrap_vectors(emb);
            ignore_result(search_repo.upsert_document(&doc).await, "backfill upsert");
            total_updated += 1;
        }

        if actual_embeddings == 0 {
            tracing::warn!("Embedding service returned no vectors, stopping backfill");
            break;
        }

        // If the raw MeiliSearch page was smaller than batch_size,
        // we've scanned the full index.
        if raw_page_size < batch_size {
            break;
        }
    }

    if total_updated > 0 {
        tracing::info!("Backfilled vectors for {} records total", total_updated);
    }
}
