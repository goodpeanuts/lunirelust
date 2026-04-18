//! Full PostgreSQL-to-MeiliSearch sync: loads all entities and upserts into the index.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::domains::search::domain::model::search_document::SearchDocument;
use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::domain::repository::tombstone_repo::TombstoneRepository as _;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::infra::tombstone_repo_impl::TombstoneRepo;
use crate::domains::search::SearchEntityType;

use super::indexer_service::{ignore_result, wrap_vectors};

/// Perform a complete PostgreSQL-to-MeiliSearch sync.
///
/// Loads every entity from `PostgreSQL`, constructs search documents, generates
/// vector embeddings for records when vLLM is available, and batch-upserts
/// into `MeiliSearch`. Tombstone versions are updated to the current timestamp
/// so that subsequent outbox events with older versions are correctly rejected.
#[expect(clippy::too_many_lines)]
pub(super) async fn run_full_sync(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::entities::{
        director, genre, idol, idol_participation, label, record, record_genre, series, studio,
    };

    // Use the current timestamp as the version for full sync so that stale
    // outbox events (with older versions) are rejected by process_upsert_event.
    let sync_version = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000);
    use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};
    use std::collections::HashMap;

    let full_sync_start = std::time::Instant::now();

    // Directors
    let directors = director::Entity::find().all(db).await?;
    for d in &directors {
        let doc = SearchDocument {
            doc_id: format!("director__{}", d.id),
            title: d.name.clone(),
            entity_type: SearchEntityType::Director,
            entity_id: d.id.to_string(),
            entity_version: sync_version,
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
        };
        search_repo.upsert_document(&doc).await?;
        ignore_result(
            TombstoneRepo::upsert_version(
                db,
                SearchEntityType::Director.as_str(),
                &d.id.to_string(),
                sync_version,
            )
            .await,
            "upsert_version director",
        );
    }
    tracing::info!(
        entity_type = "director",
        count = directors.len(),
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync: directors indexed"
    );

    // Genres
    let genres = genre::Entity::find().all(db).await?;
    for g in &genres {
        let doc = SearchDocument {
            doc_id: format!("genre__{}", g.id),
            title: g.name.clone(),
            entity_type: SearchEntityType::Genre,
            entity_id: g.id.to_string(),
            entity_version: sync_version,
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
        };
        search_repo.upsert_document(&doc).await?;
        ignore_result(
            TombstoneRepo::upsert_version(
                db,
                SearchEntityType::Genre.as_str(),
                &g.id.to_string(),
                sync_version,
            )
            .await,
            "upsert_version genre",
        );
    }
    tracing::info!(
        entity_type = "genre",
        count = genres.len(),
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync: genres indexed"
    );

    // Labels
    let labels = label::Entity::find().all(db).await?;
    for l in &labels {
        let doc = SearchDocument {
            doc_id: format!("label__{}", l.id),
            title: l.name.clone(),
            entity_type: SearchEntityType::Label,
            entity_id: l.id.to_string(),
            entity_version: sync_version,
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
        };
        search_repo.upsert_document(&doc).await?;
        ignore_result(
            TombstoneRepo::upsert_version(
                db,
                SearchEntityType::Label.as_str(),
                &l.id.to_string(),
                sync_version,
            )
            .await,
            "upsert_version label",
        );
    }
    tracing::info!(
        entity_type = "label",
        count = labels.len(),
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync: labels indexed"
    );

    // Studios
    let studios = studio::Entity::find().all(db).await?;
    for s in &studios {
        let doc = SearchDocument {
            doc_id: format!("studio__{}", s.id),
            title: s.name.clone(),
            entity_type: SearchEntityType::Studio,
            entity_id: s.id.to_string(),
            entity_version: sync_version,
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
        };
        search_repo.upsert_document(&doc).await?;
        ignore_result(
            TombstoneRepo::upsert_version(
                db,
                SearchEntityType::Studio.as_str(),
                &s.id.to_string(),
                sync_version,
            )
            .await,
            "upsert_version studio",
        );
    }
    tracing::info!(
        entity_type = "studio",
        count = studios.len(),
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync: studios indexed"
    );

    // Series
    let all_series = series::Entity::find().all(db).await?;
    for s in &all_series {
        let doc = SearchDocument {
            doc_id: format!("series__{}", s.id),
            title: s.name.clone(),
            entity_type: SearchEntityType::Series,
            entity_id: s.id.to_string(),
            entity_version: sync_version,
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
        };
        search_repo.upsert_document(&doc).await?;
        ignore_result(
            TombstoneRepo::upsert_version(
                db,
                SearchEntityType::Series.as_str(),
                &s.id.to_string(),
                sync_version,
            )
            .await,
            "upsert_version series",
        );
    }
    tracing::info!(
        entity_type = "series",
        count = all_series.len(),
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync: series indexed"
    );

    // Idols
    let idols = idol::Entity::find().all(db).await?;
    for i in &idols {
        let doc = SearchDocument {
            doc_id: format!("idol__{}", i.id),
            title: i.name.clone(),
            entity_type: SearchEntityType::Idol,
            entity_id: i.id.to_string(),
            entity_version: sync_version,
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
        };
        search_repo.upsert_document(&doc).await?;
        ignore_result(
            TombstoneRepo::upsert_version(
                db,
                SearchEntityType::Idol.as_str(),
                &i.id.to_string(),
                sync_version,
            )
            .await,
            "upsert_version idol",
        );
    }
    tracing::info!(
        entity_type = "idol",
        count = idols.len(),
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync: idols indexed"
    );

    // Records
    let records = record::Entity::find().all(db).await?;

    // Batch-load genre names per record
    let record_ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
    let all_rg = record_genre::Entity::find()
        .filter(record_genre::Column::RecordId.is_in(record_ids.clone()))
        .find_also_related(genre::Entity)
        .all(db)
        .await?;
    let genres_by_record: HashMap<String, Vec<String>> = {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for (rg, g) in all_rg {
            if let Some(genre) = g {
                map.entry(rg.record_id).or_default().push(genre.name);
            }
        }
        map
    };

    // Batch-load idol names per record
    let all_ip = idol_participation::Entity::find()
        .filter(idol_participation::Column::RecordId.is_in(record_ids))
        .find_also_related(idol::Entity)
        .all(db)
        .await?;
    let idols_by_record: HashMap<String, Vec<String>> = {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for (ip, i) in all_ip {
            if let Some(idol) = i {
                map.entry(ip.record_id).or_default().push(idol.name);
            }
        }
        map
    };

    let mut record_docs = Vec::new();
    // Build name lookups from already-loaded entity vectors to avoid N+1 queries.
    let director_map: HashMap<i64, String> =
        directors.iter().map(|d| (d.id, d.name.clone())).collect();
    let studio_map: HashMap<i64, String> = studios.iter().map(|s| (s.id, s.name.clone())).collect();
    let label_map: HashMap<i64, String> = labels.iter().map(|l| (l.id, l.name.clone())).collect();
    let series_map: HashMap<i64, String> =
        all_series.iter().map(|s| (s.id, s.name.clone())).collect();
    for r in &records {
        record_docs.push(SearchDocument {
            doc_id: format!("record__{}", r.id),
            title: r.title.clone(),
            entity_type: SearchEntityType::Record,
            entity_id: r.id.clone(),
            entity_version: sync_version,
            permission: r.permission,
            date: Some(r.date.to_string()),
            duration: Some(r.duration),
            director_name: director_map.get(&r.director_id).cloned(),
            studio_name: studio_map.get(&r.studio_id).cloned(),
            label_name: label_map.get(&r.label_id).cloned(),
            series_name: series_map.get(&r.series_id).cloned(),
            genre_names: Some(genres_by_record.get(&r.id).cloned().unwrap_or_default()),
            idol_names: Some(idols_by_record.get(&r.id).cloned().unwrap_or_default()),
            vectors: None,
        });
    }

    // Batch-generate embeddings for records when vLLM is available
    if embedding_service.is_available() && !record_docs.is_empty() {
        let titles: Vec<String> = record_docs.iter().map(|d| d.title.clone()).collect();
        let embeddings = embedding_service.embed_batch(&titles).await;
        for (doc, emb) in record_docs.iter_mut().zip(embeddings.into_iter()) {
            doc.vectors = wrap_vectors(emb);
        }
    }

    for (chunk_idx, chunk) in record_docs.chunks(100).enumerate() {
        search_repo.batch_upsert(chunk).await?;
        let indexed = std::cmp::min((chunk_idx + 1) * 100, record_docs.len());
        if (chunk_idx + 1) % 5 == 0 || indexed == record_docs.len() {
            tracing::info!(
                entity_type = "record",
                indexed,
                total = record_docs.len(),
                elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
                "Full sync: records progress"
            );
        }
    }
    for r in &records {
        ignore_result(
            TombstoneRepo::upsert_version(
                db,
                SearchEntityType::Record.as_str(),
                &r.id,
                sync_version,
            )
            .await,
            "upsert_version record",
        );
    }

    tracing::info!(
        directors = directors.len(),
        genres = genres.len(),
        labels = labels.len(),
        studios = studios.len(),
        series = all_series.len(),
        idols = idols.len(),
        records = records.len(),
        elapsed_ms = full_sync_start.elapsed().as_millis() as u64,
        "Full sync complete"
    );

    // Note: stale document cleanup is intentionally NOT done here because
    // the expected_ids snapshot is not transactionally consistent with
    // MeiliSearch. New documents can be inserted between the PostgreSQL
    // reads and the MeiliSearch fetch, causing false-positive deletions.
    // Stale documents are handled by outbox delete events and by the
    // reconciliation count check in run_indexer_loop.

    Ok(())
}
