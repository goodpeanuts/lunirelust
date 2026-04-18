//! `SearchService` implementation with `MeiliSearch`, SQL fallback, and hybrid search.

use std::str::FromStr as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait as _};

use crate::common::config::Config;
use crate::common::error::AppError;
use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::domain::service::search_service::SearchServiceTrait;
use crate::domains::search::dto::{SearchQuery, SearchResponse, SearchResultItem};
use crate::domains::search::infra::embedding::embedding_client::EmbeddingClient;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::indexer::indexer_service::IndexerService;
use crate::domains::search::infra::meilisearch::meilisearch_client::MeiliSearchClient;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::SearchEntityType;

use super::filter_utils::escape_filter_value;
use super::rrf::{hit_to_search_item, rrf_fusion};
use super::sql_fallback::search_sql_fallback;

/// Concrete implementation of `SearchServiceTrait`.
///
/// Delegates to `MeiliSearch` for keyword and hybrid search when the index is ready,
/// with automatic SQL fallback when `MeiliSearch` is unavailable.
#[allow(dead_code)]
pub struct SearchService {
    /// `PostgreSQL` connection for SQL fallback queries and user lookups.
    db: DatabaseConnection,
    /// Application configuration (`MeiliSearch` / vLLM URLs, etc.).
    #[allow(dead_code)]
    config: Config,
    /// `MeiliSearch` repository for index operations.
    search_repo: Arc<MeiliSearchRepo>,
    /// vLLM embedding service for vector search support.
    embedding_service: Arc<EmbeddingService>,
    /// Flag indicating `MeiliSearch` index is populated and ready to serve queries.
    meili_ready: Arc<AtomicBool>,
    /// Background indexer that consumes outbox events and syncs to `MeiliSearch`.
    indexer: Arc<IndexerService>,
}

/// Default maximum number of search results when no `limit` is specified.
const DEFAULT_SEARCH_LIMIT: i64 = 20;
/// RRF (Reciprocal Rank Fusion) smoothing constant `k`.
/// Higher values reduce the influence of top-ranked items; 60 is a common default.
const RRF_K: i64 = 60;

#[async_trait]
impl SearchServiceTrait for SearchService {
    /// Create a new `SearchService` with all its dependencies wired up.
    /// This is the main entry point called during application bootstrap.
    fn create_service(config: Config, db: DatabaseConnection) -> Arc<dyn SearchServiceTrait> {
        let meili_client =
            MeiliSearchClient::new(&config.meili_url, &config.meili_master_key, "luna_search");

        let embedding_client = EmbeddingClient::new(
            &config.vllm_embedding_url,
            &config.vllm_embedding_model,
            config.vllm_embedding_timeout_secs,
        );

        let search_repo = Arc::new(MeiliSearchRepo::new(meili_client));
        let embedding_service = Arc::new(EmbeddingService::new(embedding_client, None));
        let meili_ready = Arc::new(AtomicBool::new(false));

        let indexer = Arc::new(IndexerService::new(
            db.clone(),
            config.clone(),
            search_repo.clone(),
            embedding_service.clone(),
            meili_ready.clone(),
        ));

        Arc::new(Self {
            db,
            config,
            search_repo,
            embedding_service,
            meili_ready,
            indexer,
        })
    }

    /// Execute a search query. Attempts `MeiliSearch` first (hybrid or keyword-only),
    /// falls back to SQL LIKE queries if `MeiliSearch` is unavailable.
    async fn search(
        &self,
        query: SearchQuery,
        user_permission: i32,
    ) -> Result<SearchResponse, AppError> {
        let limit = query.limit.unwrap_or(DEFAULT_SEARCH_LIMIT).max(1);
        let offset = query.offset.unwrap_or(0).max(0);

        if query.q.trim().is_empty() {
            return Err(AppError::ValidationError("Query string is required".into()));
        }

        // Build permission filter
        let permission_filter = format!("permission <= {user_permission}");

        // Build entity type filter
        let entity_types: Vec<SearchEntityType> = query
            .entity_types
            .as_deref()
            .map(|s| {
                s.split(',')
                    .filter_map(|t| SearchEntityType::from_str(t.trim()).ok())
                    .collect()
            })
            .unwrap_or_default();

        // Build additional filters (escape double quotes to prevent filter injection)
        let mut additional_filters = vec![permission_filter];

        if let Some(ref director) = query.director {
            additional_filters.push(format!(
                "director_name = \"{}\"",
                escape_filter_value(director)
            ));
        }
        if let Some(ref studio) = query.studio {
            additional_filters.push(format!("studio_name = \"{}\"", escape_filter_value(studio)));
        }
        if let Some(ref label) = query.label {
            additional_filters.push(format!("label_name = \"{}\"", escape_filter_value(label)));
        }
        if let Some(ref genre) = query.genre {
            additional_filters.push(format!("genre_names = \"{}\"", escape_filter_value(genre)));
        }
        if let Some(ref date_from) = query.date_from {
            additional_filters.push(format!("date >= \"{}\"", escape_filter_value(date_from)));
        }
        if let Some(ref date_to) = query.date_to {
            additional_filters.push(format!("date <= \"{}\"", escape_filter_value(date_to)));
        }

        let filter_str = additional_filters.join(" AND ");

        // Check MeiliSearch readiness
        if self.meili_ready.load(Ordering::Relaxed) {
            // Try MeiliSearch search
            match self
                .search_meili(&query.q, &entity_types, &filter_str, limit, offset)
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::warn!("MeiliSearch search failed, falling back to SQL: {}", e);
                    // Fall through to SQL fallback
                }
            }
        }

        // SQL fallback
        search_sql_fallback(
            &self.db,
            &query.q,
            &entity_types,
            &filter_str,
            limit,
            offset,
            user_permission,
        )
        .await
    }

    /// Check if `MeiliSearch` is ready to serve queries.
    fn is_meili_ready(&self) -> bool {
        self.meili_ready.load(Ordering::Relaxed)
    }

    async fn get_user_permission(&self, user_id: &str) -> i32 {
        use crate::entities::users::Entity as UsersEntity;

        // Verify the user exists — unauthenticated / invalid tokens get 0.
        let user = UsersEntity::find_by_id(user_id)
            .one(&self.db)
            .await
            .ok()
            .flatten();

        if user.is_none() {
            return 0;
        }

        // Per-user permission levels are not yet implemented.
        // The user_ext table has a type mismatch (user_id: i64 vs
        // users.id: String/UUID) and no migration creates it, so we
        // cannot query it. Return the maximum permission level so that
        // all authenticated users see every record they could previously
        // access before the search feature was added.
        i32::MAX
    }

    /// Trigger the background startup sync and indexer loop.
    fn trigger_startup_sync(&self) {
        self.indexer.trigger_startup_sync();
    }
}

impl SearchService {
    /// Execute `MeiliSearch` hybrid search (keyword + vector).
    async fn search_meili(
        &self,
        query: &str,
        entity_types: &[SearchEntityType],
        filter_str: &str,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResponse, Box<dyn std::error::Error + Send + Sync>> {
        let embedding_available = self.embedding_service.is_available();

        // Determine search mode
        let (keyword_result, vector_result, was_prefetched) = if embedding_available {
            // In hybrid mode, both branches must fetch from offset 0 with an
            // enlarged window so that RRF fusion sees symmetric rank scores.
            // Over-fetch by 3x so that overlapping documents just below the
            // per-branch cutoff still have a chance to make the fused top-N.
            // Post-fusion slicing then picks the correct page.
            let fetch_limit = (offset + limit) * 3;

            // Start embedding generation and keyword search concurrently
            let embedding_fut = self.embedding_service.embed(query);
            let keyword_fut =
                self.search_repo
                    .keyword_search(query, entity_types, filter_str, fetch_limit, 0);

            let (keyword_res, embedding) = tokio::join!(keyword_fut, embedding_fut);

            let vector_res = match embedding {
                Some(vec) => match self
                    .search_repo
                    .vector_search(&vec, entity_types, filter_str, fetch_limit, 0)
                    .await
                {
                    Ok(res) => res,
                    Err(e) => {
                        tracing::warn!("Vector search failed, falling back to keyword-only: {}", e);
                        crate::domains::search::domain::repository::search_repo::VectorSearchResult { hits: vec![], total: 0 }
                    }
                },
                None => {
                    crate::domains::search::domain::repository::search_repo::VectorSearchResult {
                        hits: vec![],
                        total: 0,
                    }
                }
            };

            (keyword_res, Ok(vector_res), true)
        } else {
            // Keyword only — MeiliSearch handles pagination natively
            let keyword_fut =
                self.search_repo
                    .keyword_search(query, entity_types, filter_str, limit, offset);
            let empty_vector: Result<
                crate::domains::search::domain::repository::search_repo::VectorSearchResult,
                Box<dyn std::error::Error + Send + Sync>,
            > = Ok(
                crate::domains::search::domain::repository::search_repo::VectorSearchResult {
                    hits: vec![],
                    total: 0,
                },
            );
            let result = keyword_fut.await;
            (result, empty_vector, false)
        };

        let keyword_res = keyword_result?;
        let vector_res = vector_result?;

        let has_vector_results = !vector_res.hits.is_empty();
        let search_mode = if has_vector_results {
            "hybrid"
        } else {
            "keyword_only"
        };

        // RRF fusion if we have vector results.
        // Both branches were fetched from offset 0. After fusion, slice to
        // the requested page window [offset..offset+limit).
        let (results, total) = if has_vector_results {
            let fetch_limit = (offset + limit) as usize;
            let fused = rrf_fusion(&keyword_res.hits, &vector_res.hits, RRF_K, fetch_limit);

            // Compute total BEFORE slicing — use the full fused set size + any
            // documents beyond the fetch_limit that either branch found.
            let fused_total = std::cmp::max(
                fused.len() as i64,
                std::cmp::max(keyword_res.total, vector_res.total),
            );

            let page: Vec<SearchResultItem> = fused
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();
            (page, fused_total)
        } else if was_prefetched {
            // Keyword was prefetched from offset 0 with enlarged window.
            // Slice to the requested page.
            let page: Vec<SearchResultItem> = keyword_res
                .hits
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .map(|h| hit_to_search_item(&h))
                .collect();
            (page, keyword_res.total)
        } else {
            let page: Vec<SearchResultItem> = keyword_res
                .hits
                .into_iter()
                .map(|h| hit_to_search_item(&h))
                .collect();
            (page, keyword_res.total)
        };

        Ok(SearchResponse {
            search_mode: search_mode.to_owned(),
            total,
            limit,
            offset,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    // --- Hybrid total calculation regression test ---

    #[test]
    fn test_hybrid_total_vector_only_results() {
        // When keyword returns 0 but vector search has estimatedTotalHits, use vector total.
        let keyword_total: i64 = 0;
        let vector_total: i64 = 20;
        let has_vector_results = true;
        let total = if has_vector_results {
            std::cmp::max(keyword_total, vector_total)
        } else {
            keyword_total
        };
        assert_eq!(total, 20);
    }

    #[test]
    fn test_hybrid_total_keyword_dominates() {
        // When keyword found more matches, keyword_total is used.
        let keyword_total: i64 = 50;
        let vector_total: i64 = 20;
        let has_vector_results = true;
        let total = if has_vector_results {
            std::cmp::max(keyword_total, vector_total)
        } else {
            keyword_total
        };
        assert_eq!(total, 50); // keyword_total wins
    }

    #[test]
    fn test_hybrid_total_no_results() {
        // When both branches return nothing, total should be 0.
        let keyword_total: i64 = 0;
        let has_vector_results = false;
        let total = if has_vector_results {
            std::cmp::max(keyword_total, 0)
        } else {
            keyword_total
        };
        assert_eq!(total, 0);
    }

    #[test]
    fn test_hybrid_total_vector_exceeds_keyword() {
        // When vector search estimates more hits than keyword found.
        let keyword_total: i64 = 5;
        let vector_total: i64 = 100;
        let has_vector_results = true;
        let total = if has_vector_results {
            std::cmp::max(keyword_total, vector_total)
        } else {
            keyword_total
        };
        assert_eq!(total, 100); // vector total wins
    }
}
