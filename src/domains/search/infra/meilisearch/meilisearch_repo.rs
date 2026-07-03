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
/// features not yet supported by the SDK (vector search, document fetch API),
/// and for task polling to avoid the SDK's strict `Task` enum deserialization
/// which cannot handle `duration: null` returned by `MeiliSearch` v1.42.
pub struct MeiliSearchRepo {
    /// Wrapped `MeiliSearch` client with index name.
    pub(super) client: MeiliSearchClient,
    /// HTTP client used for raw API calls (vector search, fetch, task polling).
    pub(super) http: reqwest::Client,
}

const DOCUMENT_FETCH_FIELDS: [&str; 15] = [
    "id",
    "title",
    "entity_type",
    "entity_id",
    "entity_version",
    "permission",
    "date",
    "duration",
    "director_name",
    "studio_name",
    "label_name",
    "series_name",
    "genre_names",
    "idol_names",
    "_vectors",
];

impl MeiliSearchRepo {
    pub fn new(client: MeiliSearchClient) -> Self {
        Self {
            client,
            http: reqwest::Client::new(),
        }
    }

    fn bearer_token(&self) -> String {
        format!(
            "Bearer {}",
            self.client.client.get_api_key().unwrap_or_default()
        )
    }

    fn documents_fetch_url(&self) -> String {
        format!(
            "{}/indexes/{}/documents/fetch",
            self.client.client.get_host().trim_end_matches('/'),
            self.client.index_name
        )
    }

    async fn documents_fetch(
        &self,
        body: JsonValue,
    ) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .http
            .post(self.documents_fetch_url())
            .header("Authorization", self.bearer_token())
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("MeiliSearch documents fetch failed {status}: {text}").into());
        }

        Ok(resp.json().await?)
    }

    async fn exact_document_count_with_filter(
        &self,
        filter: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        const BATCH_SIZE: usize = 1000;

        let mut offset = 0usize;
        let mut counted = 0u64;

        loop {
            let response = self
                .documents_fetch(json!({
                    "filter": filter,
                    "offset": offset,
                    "limit": BATCH_SIZE,
                    "fields": ["id"]
                }))
                .await?;

            if let Some(total) = extract_fetch_total(&response) {
                return Ok(total);
            }

            let page_size = extract_fetch_results_len(&response);
            counted += page_size as u64;

            if page_size < BATCH_SIZE {
                return Ok(counted);
            }

            offset += BATCH_SIZE;
        }
    }

    pub(super) async fn normalize_documents_for_user_provided_embedder(
        &self,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        const BATCH_SIZE: usize = 1000;
        const UPSERT_BATCH_SIZE: usize = 100;

        let mut offset = 0usize;
        let mut normalized_count = 0usize;

        loop {
            // `_vectors` is protected (MeiliSearch >= v1.11): require
            // `retrieveVectors: true` so `document_missing_default_vector` can
            // tell which documents already have vectors and must NOT be
            // rewritten with `{"default": null}` (which would strip existing
            // embeddings during embedder normalization).
            let response = self
                .documents_fetch(json!({
                    "offset": offset,
                    "limit": BATCH_SIZE,
                    "fields": DOCUMENT_FETCH_FIELDS,
                    "retrieveVectors": true
                }))
                .await?;

            let results = response
                .get("results")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();

            if results.is_empty() {
                break;
            }

            let filtered: Vec<SearchDocument> = results
                .iter()
                .filter(|doc| document_missing_default_vector(doc))
                .filter_map(|doc| serde_json::from_value(doc.clone()).ok())
                .collect();

            normalized_count += filtered.len();

            for chunk in filtered.chunks(UPSERT_BATCH_SIZE) {
                self.batch_upsert(chunk).await?;
            }

            let page_size = results.len();
            if page_size < BATCH_SIZE {
                break;
            }

            offset += page_size;
        }

        Ok(normalized_count)
    }

    /// Poll a `MeiliSearch` task until it reaches a terminal state.
    ///
    /// Uses raw HTTP + `serde_json::Value` instead of the SDK's `wait_for_task`
    /// because the SDK's `Task` enum cannot deserialize `duration: null` — a
    /// condition that `MeiliSearch` v1.42 occasionally produces when its
    /// index-scheduler records a stale `finishedAt` timestamp.
    pub(super) async fn wait_for_task_with_debug(
        &self,
        task_uid: u32,
        operation: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let base_url = self.client.client.get_host();
        let api_key = self.client.client.get_api_key().unwrap_or_default();
        let task_url = format!("{}/tasks/{}", base_url.trim_end_matches('/'), task_uid);

        let poll_interval = std::time::Duration::from_millis(200);
        let timeout = std::time::Duration::from_secs(30);
        let start = std::time::Instant::now();

        loop {
            let resp = self
                .http
                .get(&task_url)
                .header("Authorization", format!("Bearer {api_key}"))
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("MeiliSearch task poll HTTP error {status}: {body}").into());
            }

            let task_value: JsonValue = resp.json().await?;

            match task_value
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
            {
                "succeeded" => return Ok(()),
                "failed" => {
                    let error_msg = extract_task_error(&task_value);
                    tracing::error!(
                        operation,
                        task_uid,
                        error_msg,
                        raw_response = %task_value,
                        "MeiliSearch task failed"
                    );
                    return Err(format!("MeiliSearch task {task_uid} failed: {error_msg}").into());
                }
                "canceled" => {
                    return Err(format!("MeiliSearch task {task_uid} was canceled").into());
                }
                _ => {}
            }

            if start.elapsed() >= timeout {
                let last_status = task_value
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                tracing::error!(
                    operation,
                    task_uid,
                    last_status,
                    "MeiliSearch task wait timed out"
                );
                return Err(format!(
                    "MeiliSearch task {task_uid} timed out after {timeout:?} (last status: {last_status})"
                )
                .into());
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}

/// Extract a human-readable error message from a failed task's JSON.
fn extract_task_error(task: &JsonValue) -> String {
    if let Some(error) = task.get("error") {
        if let Some(message) = error.get("message").and_then(|m| m.as_str()) {
            return message.to_owned();
        }
        return serde_json::to_string(error).unwrap_or_else(|_| "unknown error".to_owned());
    }
    "no error details in response".to_owned()
}

fn extract_fetch_total(response: &JsonValue) -> Option<u64> {
    response.get("total").and_then(|value| value.as_u64())
}

fn extract_fetch_results_len(response: &JsonValue) -> usize {
    response
        .get("results")
        .and_then(|value| value.as_array())
        .map_or(0, Vec::len)
}

fn document_missing_default_vector(doc: &JsonValue) -> bool {
    let Some(default) = doc.get("_vectors").and_then(|v| v.get("default")) else {
        return true;
    };

    if default.is_null() {
        return true;
    }

    // MeiliSearch >= v1.11 stores vectors as
    // `_vectors.default = {"embeddings": [[...]], "regenerate": bool}`. A
    // document is missing its embedding when that array is absent or empty.
    if let Some(embeddings) = default.get("embeddings") {
        return embeddings.as_array().is_none_or(Vec::is_empty);
    }

    // Legacy format (pre-v1.11): `_vectors.default` is the vector array itself.
    default.as_array().is_none_or(Vec::is_empty)
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
        self.wait_for_task_with_debug(task.get_task_uid(), "upsert_document")
            .await?;
        Ok(())
    }

    async fn delete_document(
        &self,
        doc_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let index = self.client.index();
        let task = index.delete_document(doc_id).await?;
        self.wait_for_task_with_debug(task.get_task_uid(), "delete_document")
            .await?;
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
        self.wait_for_task_with_debug(task.get_task_uid(), "batch_upsert")
            .await?;
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
            self.exact_document_count_with_filter(&filter).await
        }
    }

    async fn find_records_missing_vectors(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SearchDocument>, usize), Box<dyn std::error::Error + Send + Sync>> {
        // `_vectors` is a protected field: MeiliSearch (>= v1.11) omits it from
        // `/documents/fetch` responses unless `retrieveVectors: true` is set, even
        // when it is listed in `fields`. Without this flag every document is
        // falsely judged missing its vector and gets re-embedded each cycle.
        let json = self
            .documents_fetch(json!({
            "filter": "entity_type = \"record\"",
            "offset": offset,
            "limit": limit,
            "fields": DOCUMENT_FETCH_FIELDS,
            "retrieveVectors": true
            }))
            .await
            .map_err(|error| {
                tracing::debug!("find_records_missing_vectors: {}", error);
                error
            })?;
        let results = json
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        let raw_page_size = results.len();

        let docs: Vec<SearchDocument> = results
            .iter()
            .filter(|doc| document_missing_default_vector(doc))
            .filter_map(|doc| serde_json::from_value(doc.clone()).ok())
            .collect();

        Ok((docs, raw_page_size))
    }

    async fn get_entity_ids(
        &self,
        entity_type: SearchEntityType,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut all_ids = Vec::new();
        let mut offset: usize = 0;
        let batch: usize = 1000;

        loop {
            let json = match self
                .documents_fetch(json!({
                    "filter": format!("entity_type = \"{}\"", entity_type.as_str()),
                    "offset": offset,
                    "limit": batch,
                    "fields": ["entity_id"]
                }))
                .await
            {
                Ok(json) => json,
                Err(error) => {
                    tracing::debug!("get_entity_ids: {}", error);
                    break;
                }
            };
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
    fn test_extract_task_error_with_message() {
        let task = json!({
            "status": "failed",
            "error": {
                "message": "document id is invalid",
                "code": "invalid_document_id",
                "type": "invalid_request"
            }
        });
        assert_eq!(extract_task_error(&task), "document id is invalid");
    }

    #[test]
    fn test_extract_task_error_without_message() {
        let task = json!({
            "status": "failed",
            "error": {"code": "internal", "type": "internal"}
        });
        let result = extract_task_error(&task);
        assert!(result.contains("internal"));
    }

    #[test]
    fn test_extract_task_error_no_error_field() {
        let task = json!({"status": "succeeded"});
        assert_eq!(extract_task_error(&task), "no error details in response");
    }

    #[test]
    fn test_extract_fetch_total_uses_exact_total_for_large_dataset() {
        let response = json!({
            "results": [
                {"id": "record__1"}
            ],
            "offset": 0,
            "limit": 1,
            "total": 1095
        });

        assert_eq!(extract_fetch_total(&response), Some(1095));
    }

    #[test]
    fn test_extract_fetch_total_returns_none_when_total_missing() {
        let response = json!({
            "results": [
                {"id": "record__1"}
            ],
            "offset": 0,
            "limit": 1
        });

        assert_eq!(extract_fetch_total(&response), None);
    }

    #[test]
    fn test_missing_vector_detection_treats_explicit_opt_out_as_missing() {
        let doc = json!({
            "id": "record__1",
            "_vectors": {
                "default": null
            }
        });

        assert!(document_missing_default_vector(&doc));
    }

    #[test]
    fn test_missing_vector_detection_treats_actual_vector_as_present() {
        let doc = json!({
            "id": "record__1",
            "_vectors": {
                "default": [0.1, 0.2, 0.3]
            }
        });

        assert!(!document_missing_default_vector(&doc));
    }

    #[test]
    fn test_missing_vector_detection_absent_vectors_is_missing() {
        let doc = json!({ "id": "record__1" });
        assert!(document_missing_default_vector(&doc));
    }

    #[test]
    fn test_missing_vector_detection_v1_11_empty_embeddings_is_missing() {
        // MeiliSearch >= v1.11 stores an opted-out document as
        // `{"embeddings": [], "regenerate": false}`; this MUST read as missing.
        let doc = json!({
            "id": "record__1",
            "_vectors": {
                "default": { "embeddings": [], "regenerate": false }
            }
        });

        assert!(document_missing_default_vector(&doc));
    }

    #[test]
    fn test_missing_vector_detection_v1_11_nonempty_embeddings_is_present() {
        let doc = json!({
            "id": "record__1",
            "_vectors": {
                "default": { "embeddings": [[0.1, 0.2, 0.3]], "regenerate": false }
            }
        });

        assert!(!document_missing_default_vector(&doc));
    }

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

    // --- MeiliSearch integration tests ---
    //
    // These require a live MeiliSearch instance (started by `just test-db`,
    // mapped to MEILI_URL in .env.test). They auto-skip when the service is
    // unreachable so bare `cargo nextest run` without the compose stack does
    // not fail. Each test uses an isolated index name to avoid interference.

    /// Embedder dimension configured by `index_setup` (BGE-M3).
    const TEST_EMBEDDING_DIM: usize = 1024;

    /// Build a repo against an isolated test index, wiping any leftover index
    /// first and configuring it via `init_index` so `_vectors` is meaningful.
    /// Returns `None` (skip) when `MeiliSearch` is not reachable.
    async fn integration_repo(index_name: &str) -> Option<MeiliSearchRepo> {
        let url = std::env::var("MEILI_URL").unwrap_or_else(|_| "http://localhost:7701".to_owned());
        let key = std::env::var("MEILI_MASTER_KEY")
            .unwrap_or_else(|_| "meili_master_key_test".to_owned());
        let client = MeiliSearchClient::new(&url, &key, index_name);
        if !client.health_check().await {
            tracing::info!("skipping MeiliSearch integration test: service unavailable");
            return None;
        }
        let repo = MeiliSearchRepo::new(client);
        // Wipe any leftover index from a prior run so the test starts clean.
        if let Ok(task) = repo.client.client.delete_index(index_name).await {
            if let Err(e) = repo
                .wait_for_task_with_debug(task.get_task_uid(), "test_delete_index")
                .await
            {
                tracing::debug!("test index cleanup wait failed: {e}");
            }
        }
        repo.init_index()
            .await
            .expect("init_index must configure the embedder");
        Some(repo)
    }

    /// Build a minimal record document, optionally carrying a default-embedder
    /// vector of zeros.
    fn record_doc(id: &str, vectors: Option<JsonValue>) -> SearchDocument {
        SearchDocument {
            doc_id: format!("record__{id}"),
            title: format!("title-{id}"),
            entity_type: SearchEntityType::Record,
            entity_id: id.to_owned(),
            entity_version: 1,
            permission: 0,
            date: None,
            duration: None,
            director_name: None,
            studio_name: None,
            label_name: None,
            series_name: None,
            genre_names: None,
            idol_names: None,
            vectors,
        }
    }

    #[tokio::test]
    async fn integration_find_records_missing_vectors_only_returns_vector_less() {
        let Some(repo) = integration_repo("luna_search_test_rv_find").await else {
            return;
        };
        let with_vector = record_doc(
            "with-vec",
            Some(json!({ "default": vec![0.0_f64; TEST_EMBEDDING_DIM] })),
        );
        let without_vector = record_doc("no-vec", None);

        repo.batch_upsert(&[with_vector, without_vector])
            .await
            .expect("upsert test docs");

        let (missing, _raw_page_size) = repo
            .find_records_missing_vectors(0, 50)
            .await
            .expect("find_records_missing_vectors");

        let missing_ids: Vec<&str> = missing.iter().map(|d| d.doc_id.as_str()).collect();
        assert!(
            missing_ids.contains(&"record__no-vec"),
            "record without a vector must be reported missing: {missing_ids:?}"
        );
        assert!(
            !missing_ids.contains(&"record__with-vec"),
            "record with a vector must NOT be reported missing: {missing_ids:?}"
        );
    }

    #[tokio::test]
    async fn integration_normalize_does_not_strip_existing_vectors() {
        let Some(repo) = integration_repo("luna_search_test_rv_norm").await else {
            return;
        };
        let with_vector = record_doc(
            "keep-vec",
            Some(json!({ "default": vec![0.0_f64; TEST_EMBEDDING_DIM] })),
        );
        repo.batch_upsert(&[with_vector])
            .await
            .expect("upsert test doc");

        // The normalization pass must leave documents that already have a
        // vector untouched (i.e. rewrite zero documents).
        let normalized = repo
            .normalize_documents_for_user_provided_embedder()
            .await
            .expect("normalize");
        assert_eq!(
            normalized, 0,
            "a document that already has a vector must not be rewritten"
        );

        // Confirm the vector survived: the document must not be reported missing.
        let (missing, _) = repo
            .find_records_missing_vectors(0, 50)
            .await
            .expect("find_records_missing_vectors");
        let still_missing: Vec<&str> = missing.iter().map(|d| d.doc_id.as_str()).collect();
        assert!(
            !still_missing.contains(&"record__keep-vec"),
            "normalize must not strip an existing vector: {still_missing:?}"
        );
    }
}
