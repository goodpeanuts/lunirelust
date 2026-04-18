//! `MeiliSearch` repository implementation for `SearchRepository` trait.

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};

use crate::domains::search::domain::model::search_document::SearchDocument;
use crate::domains::search::domain::repository::search_repo::{
    KeywordSearchResult, SearchHit, SearchRepository, VectorSearchResult,
};
use crate::domains::search::infra::impl_service::filter_utils::escape_filter_value;
use crate::domains::search::infra::meilisearch::meilisearch_client::MeiliSearchClient;
use crate::domains::search::SearchEntityType;

/// `MeiliSearch` repository implementing the `SearchRepository` trait.
///
/// Uses the `MeiliSearch` SDK for standard operations and raw HTTP for
/// features not yet supported by the SDK (vector search, document fetch API).
pub struct MeiliSearchRepo {
    /// Wrapped `MeiliSearch` client with index name.
    pub(super) client: MeiliSearchClient,
    /// HTTP client used for raw API calls (vector search, fetch, etc.).
    pub(super) http: reqwest::Client,
}

impl MeiliSearchRepo {
    pub fn new(client: MeiliSearchClient) -> Self {
        Self {
            client,
            http: reqwest::Client::new(),
        }
    }
}

/// Check that a `MeiliSearch` async task completed successfully.
///
/// The SDK's `wait_for_task` returns `Ok(Task::Failed{..})` rather than `Err`
/// when the task fails server-side, so we must inspect the variant to detect
/// errors like `invalid_document_id`.
fn check_task_success(
    task: meilisearch_sdk::tasks::Task,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match task {
        meilisearch_sdk::tasks::Task::Failed { content } => {
            Err(format!("MeiliSearch task failed: {:?}", content.error).into())
        }
        _ => Ok(()),
    }
}

#[async_trait]
impl SearchRepository for MeiliSearchRepo {
    async fn init_index(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        super::index_setup::init_index(self).await
    }

    async fn health_check(&self) -> bool {
        self.client.health_check().await
    }

    async fn upsert_document(
        &self,
        doc: &SearchDocument,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let index = self.client.index();
        let task = index.add_documents(&[doc], Some("id")).await?;
        check_task_success(self.client.client.wait_for_task(task, None, None).await?)?;
        Ok(())
    }

    async fn delete_document(
        &self,
        doc_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let index = self.client.index();
        let task = index.delete_document(doc_id).await?;
        check_task_success(self.client.client.wait_for_task(task, None, None).await?)?;
        Ok(())
    }

    async fn batch_upsert(
        &self,
        docs: &[SearchDocument],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if docs.is_empty() {
            return Ok(());
        }
        let index = self.client.index();
        let task = index.add_documents(docs, Some("id")).await?;
        check_task_success(self.client.client.wait_for_task(task, None, None).await?)?;
        Ok(())
    }

    async fn keyword_search(
        &self,
        query: &str,
        entity_types: &[SearchEntityType],
        filters: &str,
        limit: i64,
        offset: i64,
    ) -> Result<KeywordSearchResult, Box<dyn std::error::Error + Send + Sync>> {
        let index = self.client.index();
        let filter_str = build_filter_string(entity_types, filters);

        let mut search = index.search();
        search.query = Some(query);
        search.limit = Some(limit as usize);
        search.offset = Some(offset as usize);
        if !filter_str.is_empty() {
            search.with_filter(&filter_str);
        }
        search.with_attributes_to_highlight(meilisearch_sdk::search::Selectors::Some(&["title"]));
        search.show_ranking_score = Some(true);

        let results = index.execute_query::<JsonValue>(&search).await?;

        let total = results.estimated_total_hits.unwrap_or(0) as i64;
        let hits = results
            .hits
            .into_iter()
            .map(|hit| {
                let doc_id = hit
                    .result
                    .get("doc_id")
                    .or(hit.result.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned();
                SearchHit {
                    doc_id,
                    score: hit.ranking_score.unwrap_or(0.0),
                    document: hit.result,
                    formatted: hit.formatted_result.map(serde_json::Value::Object),
                }
            })
            .collect();

        Ok(KeywordSearchResult { total, hits })
    }

    async fn vector_search(
        &self,
        vector: &[f32],
        entity_types: &[SearchEntityType],
        filters: &str,
        limit: i64,
        offset: i64,
    ) -> Result<VectorSearchResult, Box<dyn std::error::Error + Send + Sync>> {
        // Use MeiliSearch's raw search API for vector search since the SDK
        // (0.28) does not expose hybrid/vector parameters.
        let search_url = format!(
            "{}/indexes/{}/search",
            self.client.client.get_host(),
            self.client.index_name
        );

        let combined_filter = build_filter_string(entity_types, filters);

        let mut body = json!({
            "q": "",
            "vector": vector,
            "hybrid": {
                "embedder": "default"
            },
            "limit": limit,
            "offset": offset,
            "showRankingScore": true,
        });

        if !combined_filter.is_empty() {
            body["filter"] = json!(combined_filter);
        }

        let http_client = &self.http;
        let resp = http_client
            .post(&search_url)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    self.client.client.get_api_key().unwrap_or_default()
                ),
            )
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::debug!("Vector search failed ({}): {}", status, text);
            return Ok(VectorSearchResult {
                hits: vec![],
                total: 0,
            });
        }

        let result: JsonValue = resp.json().await?;
        let total = result
            .get("estimatedTotalHits")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as i64;
        let hits = result
            .get("hits")
            .and_then(|h| h.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|hit| {
                        let doc_id = hit
                            .get("id")
                            .or(hit.get("doc_id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_owned();
                        let score = hit
                            .get("_rankingScore")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        SearchHit {
                            doc_id,
                            score,
                            document: hit.clone(),
                            formatted: None,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(VectorSearchResult { hits, total })
    }

    async fn get_document_count(
        &self,
        entity_type: SearchEntityType,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let index = self.client.index();
        let stats = index.get_stats().await?;

        if entity_type.as_str().is_empty() || entity_type.as_str() == "*" {
            Ok(stats.number_of_documents as u64)
        } else {
            let filter = format!("entity_type = \"{}\"", entity_type.as_str());
            let mut sq = index.search();
            sq.query = Some("");
            sq.limit = Some(0);
            sq.with_filter(&filter);
            let results = index.execute_query::<JsonValue>(&sq).await?;
            Ok(results.estimated_total_hits.unwrap_or(0) as u64)
        }
    }

    async fn find_records_missing_vectors(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SearchDocument>, usize), Box<dyn std::error::Error + Send + Sync>> {
        let fetch_url = format!(
            "{}/indexes/{}/documents/fetch",
            self.client.client.get_host(),
            self.client.index_name
        );
        let http_client = &self.http;

        // Fetch record documents including _vectors field so we can check for
        // missing embeddings client-side. We cannot filter on _vectors because
        // it is not declared as a filterable attribute.
        let body = json!({
            "filter": "entity_type = \"record\"",
            "offset": offset,
            "limit": limit,
            "fields": ["id", "title", "entity_type", "entity_id", "entity_version",
                        "permission", "date", "duration", "director_name", "studio_name",
                        "label_name", "series_name", "genre_names", "idol_names", "_vectors"]
        });

        let resp = http_client
            .post(&fetch_url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    self.client.client.get_api_key().unwrap_or_default()
                ),
            )
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::debug!("find_records_missing_vectors: {} {}", status, text);
            return Ok((vec![], 0));
        }

        let json: JsonValue = resp.json().await?;
        let results = json
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        let raw_page_size = results.len();

        // Filter client-side: only return documents without vectors.
        // _vectors is a MeiliSearch-managed field that may be null, missing,
        // or contain {"default": {"embeddings": [...]}}.
        let docs: Vec<SearchDocument> = results
            .iter()
            .filter(|doc| {
                let vectors = doc.get("_vectors");
                vectors.is_none() || vectors == Some(&JsonValue::Null)
            })
            .filter_map(|doc| serde_json::from_value(doc.clone()).ok())
            .collect();

        Ok((docs, raw_page_size))
    }

    async fn get_entity_ids(
        &self,
        entity_type: SearchEntityType,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let fetch_url = format!(
            "{}/indexes/{}/documents/fetch",
            self.client.client.get_host(),
            self.client.index_name
        );
        let http_client = &self.http;
        let mut all_ids = Vec::new();
        let mut offset: usize = 0;
        let batch: usize = 1000;

        loop {
            let body = json!({
                "filter": format!("entity_type = \"{}\"", entity_type.as_str()),
                "offset": offset,
                "limit": batch,
                "fields": ["entity_id"]
            });
            let resp = http_client
                .post(&fetch_url)
                .header(
                    "Authorization",
                    format!(
                        "Bearer {}",
                        self.client.client.get_api_key().unwrap_or_default()
                    ),
                )
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                tracing::debug!("get_entity_ids: {} {}", status, text);
                break;
            }

            let json: JsonValue = resp.json().await?;
            let results = json.get("results").and_then(|r| r.as_array());
            match results {
                Some(arr) if arr.is_empty() => break,
                Some(arr) => {
                    for doc in arr {
                        if let Some(id) = doc.get("entity_id").and_then(|v| v.as_str()) {
                            all_ids.push(id.to_owned());
                        }
                    }
                    if arr.len() < batch {
                        break;
                    }
                    offset += batch;
                }
                None => break,
            }
        }

        Ok(all_ids)
    }
}

/// Build a `MeiliSearch` filter string combining entity type OR-clauses with additional AND filters.
fn build_filter_string(entity_types: &[SearchEntityType], additional_filters: &str) -> String {
    let mut parts = Vec::new();

    if !entity_types.is_empty() {
        let type_filters: Vec<String> = entity_types
            .iter()
            .map(|t| format!("entity_type = \"{}\"", escape_filter_value(t.as_str())))
            .collect();
        parts.push(format!("({})", type_filters.join(" OR ")));
    }

    if !additional_filters.is_empty() {
        parts.push(additional_filters.to_owned());
    }

    parts.join(" AND ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_empty() {
        let filter = build_filter_string(&[], "");
        assert!(filter.is_empty());
    }

    #[test]
    fn test_build_filter_entity_types_only() {
        let filter = build_filter_string(&[SearchEntityType::Record, SearchEntityType::Idol], "");
        assert_eq!(
            filter,
            "(entity_type = \"record\" OR entity_type = \"idol\")"
        );
    }

    #[test]
    fn test_build_filter_additional_only() {
        let filter = build_filter_string(&[], "permission <= 5");
        assert_eq!(filter, "permission <= 5");
    }

    #[test]
    fn test_build_filter_combined() {
        let filter = build_filter_string(&[SearchEntityType::Record], "permission <= 5");
        assert_eq!(filter, "(entity_type = \"record\") AND permission <= 5");
    }

    #[test]
    fn test_build_filter_multiple_entity_types_and_filter() {
        let filter = build_filter_string(
            &[SearchEntityType::Record, SearchEntityType::Idol],
            "date >= \"2024-01-01\"",
        );
        assert_eq!(
            filter,
            "(entity_type = \"record\" OR entity_type = \"idol\") AND date >= \"2024-01-01\""
        );
    }
}
