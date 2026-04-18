//! Embedding service with health check, availability tracking, and batch support.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::domains::search::infra::embedding::embedding_client::EmbeddingClient;

/// Default batch size for embedding generation.
const DEFAULT_BATCH_SIZE: usize = 32;

/// Embedding service that wraps the vLLM client with availability tracking.
///
/// The service starts as unavailable and must pass a health check before
/// embedding requests are forwarded to vLLM. On failure, it marks itself
/// unavailable until the next successful health check.
#[derive(Clone)]
pub struct EmbeddingService {
    /// Underlying vLLM HTTP client.
    client: EmbeddingClient,
    /// Current availability flag, updated by health checks and embed failures.
    available: Arc<AtomicBool>,
    /// Maximum number of texts per batch embedding request.
    #[allow(dead_code)]
    batch_size: usize,
}

impl EmbeddingService {
    /// Create a new embedding service. The service starts as unavailable;
    /// call `check_health()` or `set_available(true)` to enable it.
    pub fn new(client: EmbeddingClient, batch_size: Option<usize>) -> Self {
        Self {
            client,
            available: Arc::new(AtomicBool::new(false)),
            batch_size: batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
        }
    }

    /// Generate an embedding for a single text.
    pub async fn embed(&self, text: &str) -> Option<Vec<f32>> {
        if !self.available.load(Ordering::Relaxed) {
            return None;
        }
        match self.client.embed(text).await {
            Ok(embedding) => Some(embedding),
            Err(e) => {
                tracing::warn!("Embedding generation failed: {}", e);
                self.available.store(false, Ordering::Relaxed);
                None
            }
        }
    }

    /// Generate embeddings for a batch of texts.
    #[allow(dead_code)]
    pub async fn embed_batch(&self, texts: &[String]) -> Vec<Option<Vec<f32>>> {
        if !self.available.load(Ordering::Relaxed) || texts.is_empty() {
            return texts.iter().map(|_| None).collect();
        }

        // Process in chunks of batch_size
        let mut results: Vec<Option<Vec<f32>>> = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(self.batch_size) {
            match self.client.embed_batch(chunk).await {
                Ok(embeddings) => {
                    for embedding in embeddings {
                        results.push(Some(embedding));
                    }
                }
                Err(e) => {
                    tracing::warn!("Batch embedding failed: {}", e);
                    self.available.store(false, Ordering::Relaxed);
                    for _ in chunk {
                        results.push(None);
                    }
                    return results;
                }
            }
        }
        results
    }

    /// Check health and update availability status.
    pub async fn check_health(&self) -> bool {
        let healthy = self.client.health_check().await;
        let was_available = self.available.load(Ordering::Relaxed);

        self.available.store(healthy, Ordering::Relaxed);

        if healthy && !was_available {
            tracing::info!("vLLM embedding service recovered");
        } else if !healthy && was_available {
            tracing::warn!("vLLM embedding service became unavailable");
        }

        healthy
    }

    /// Check if the embedding service is currently available.
    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }

    /// Force set availability (useful for startup).
    #[allow(dead_code)]
    pub fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> EmbeddingService {
        let client = EmbeddingClient::new("http://localhost:9999", "test-model", 5);
        EmbeddingService::new(client, Some(16))
    }

    #[test]
    fn test_embedding_service_starts_unavailable() {
        let svc = make_service();
        assert!(!svc.is_available());
    }

    #[test]
    fn test_set_available() {
        let svc = make_service();
        assert!(!svc.is_available());
        svc.set_available(true);
        assert!(svc.is_available());
        svc.set_available(false);
        assert!(!svc.is_available());
    }

    #[test]
    fn test_default_batch_size() {
        let client = EmbeddingClient::new("http://localhost:9999", "m", 5);
        let svc = EmbeddingService::new(client, None);
        assert!(!svc.is_available());
    }

    #[tokio::test]
    async fn test_embed_returns_none_when_unavailable() {
        let svc = make_service();
        let result = svc.embed("test").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_embed_batch_returns_none_when_unavailable() {
        let svc = make_service();
        let texts = vec!["a".to_owned(), "b".to_owned()];
        let results = svc.embed_batch(&texts).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_none()));
    }

    #[tokio::test]
    async fn test_embed_batch_empty_input() {
        let svc = make_service();
        svc.set_available(true);
        let texts: Vec<String> = vec![];
        let results = svc.embed_batch(&texts).await;
        assert!(results.is_empty());
    }

    #[test]
    fn test_availability_transition() {
        let svc = make_service();
        // Starts unavailable
        assert!(!svc.is_available());
        // Can be set to available
        svc.set_available(true);
        assert!(svc.is_available());
        // Can flip back
        svc.set_available(false);
        assert!(!svc.is_available());
    }

    #[test]
    fn test_custom_batch_size() {
        let client = EmbeddingClient::new("http://localhost:9999", "m", 5);
        let svc = EmbeddingService::new(client, Some(8));
        // batch_size is stored; just verify construction doesn't panic
        assert!(!svc.is_available());
    }
}
