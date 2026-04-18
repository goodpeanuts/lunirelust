//! `SearchRepository` trait for `MeiliSearch` document operations.

use crate::domains::search::domain::model::search_document::SearchDocument;
use crate::domains::search::SearchEntityType;
use async_trait::async_trait;

/// Repository trait for search document operations against `MeiliSearch`.
#[async_trait]
pub trait SearchRepository: Send + Sync {
    /// Initialize the `MeiliSearch` index with proper settings.
    async fn init_index(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Check if `MeiliSearch` is healthy.
    async fn health_check(&self) -> bool;

    /// Upsert a single document into the index.
    async fn upsert_document(
        &self,
        doc: &SearchDocument,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Delete a document from the index by its document ID.
    async fn delete_document(
        &self,
        doc_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Batch upsert multiple documents.
    async fn batch_upsert(
        &self,
        docs: &[SearchDocument],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Execute a keyword search with filters and pagination.
    async fn keyword_search(
        &self,
        query: &str,
        entity_types: &[SearchEntityType],
        filters: &str,
        limit: i64,
        offset: i64,
    ) -> Result<KeywordSearchResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Execute a vector search using embeddings.
    async fn vector_search(
        &self,
        vector: &[f32],
        entity_types: &[SearchEntityType],
        filters: &str,
        limit: i64,
        offset: i64,
    ) -> Result<VectorSearchResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the total number of documents in the index for a given entity type.
    async fn get_document_count(
        &self,
        entity_type: SearchEntityType,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Find record documents missing vector embeddings.
    #[allow(dead_code)]
    async fn find_records_missing_vectors(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SearchDocument>, usize), Box<dyn std::error::Error + Send + Sync>>;

    /// Get all entity IDs from `MeiliSearch` for a given entity type.
    /// Used by reconciliation to detect ghost documents.
    async fn get_entity_ids(
        &self,
        entity_type: SearchEntityType,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Result from a keyword search.
#[derive(Debug)]
pub struct KeywordSearchResult {
    /// Total number of matching documents (estimated by `MeiliSearch`).
    pub total: i64,
    /// Matching documents with scores and highlights.
    pub hits: Vec<SearchHit>,
}

/// Result from a vector search.
#[derive(Debug)]
pub struct VectorSearchResult {
    /// Matching documents with scores.
    pub hits: Vec<SearchHit>,
    /// Total number of matching documents (estimated by `MeiliSearch`).
    pub total: i64,
}

/// A single search hit from `MeiliSearch`.
#[derive(Debug, Clone)]
pub struct SearchHit {
    /// Composite document ID: "{`entity_type`}__{`entity_id`}".
    pub doc_id: String,
    /// Relevance score assigned by `MeiliSearch`.
    pub score: f64,
    /// Raw document fields as JSON.
    pub document: serde_json::Value,
    /// Formatted document with highlighted snippets (keyword search only).
    pub formatted: Option<serde_json::Value>,
}
