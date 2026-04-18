//! vLLM embedding client for generating vector embeddings.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Request body for vLLM /v1/embeddings API.
#[derive(Serialize)]
struct EmbeddingRequest {
    /// Model identifier to use for embedding generation.
    model: String,
    /// Input text: single string or batch array.
    input: EmbeddingInput,
}

/// Input can be a single string or array of strings.
#[derive(Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum EmbeddingInput {
    /// Single text input.
    Single(String),
    /// Batch of text inputs.
    Batch(Vec<String>),
}

/// Response from vLLM /v1/embeddings API.
#[derive(Deserialize)]
struct EmbeddingResponse {
    /// List of embedding results, one per input.
    data: Vec<EmbeddingData>,
}

/// Single embedding result from the vLLM API.
#[derive(Deserialize)]
struct EmbeddingData {
    /// The embedding vector.
    embedding: Vec<f32>,
}

/// Client for vLLM embedding API.
#[derive(Clone)]
pub struct EmbeddingClient {
    /// HTTP client with configured timeout.
    http: Client,
    /// Base URL of the vLLM service (trailing slash stripped).
    base_url: String,
    /// Model identifier for embedding generation.
    model: String,
    /// Request timeout duration.
    timeout: Duration,
}

impl EmbeddingClient {
    pub fn new(base_url: &str, model: &str, timeout_secs: u64) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_owned(),
            model: model.to_owned(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Generate an embedding for a single text string.
    pub async fn embed(
        &self,
        text: &str,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let req = EmbeddingRequest {
            model: self.model.clone(),
            input: EmbeddingInput::Single(text.to_owned()),
        };

        let resp = self
            .http
            .post(format!("{}/v1/embeddings", self.base_url))
            .json(&req)
            .timeout(self.timeout)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("vLLM embedding error: {status} - {body}").into());
        }

        let data: EmbeddingResponse = resp.json().await?;
        data.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| "No embedding data returned from vLLM".into())
    }

    /// Generate embeddings for a batch of text strings.
    #[allow(dead_code)]
    pub async fn embed_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error + Send + Sync>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let req = EmbeddingRequest {
            model: self.model.clone(),
            input: EmbeddingInput::Batch(texts.to_vec()),
        };

        let resp = self
            .http
            .post(format!("{}/v1/embeddings", self.base_url))
            .json(&req)
            .timeout(self.timeout)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("vLLM batch embedding error: {status} - {body}").into());
        }

        let data: EmbeddingResponse = resp.json().await?;
        // API returns embeddings in input order
        Ok(data.data.into_iter().map(|d| d.embedding).collect())
    }

    /// Check if the vLLM embedding service is healthy.
    pub async fn health_check(&self) -> bool {
        match self
            .http
            .get(format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                tracing::debug!("vLLM health check failed: {}", e);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_client_new_trims_trailing_slash() {
        let client = EmbeddingClient::new("http://localhost:8000/", "model", 30);
        assert_eq!(client.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_embedding_client_new_no_trailing_slash() {
        let client = EmbeddingClient::new("http://localhost:8000", "model", 30);
        assert_eq!(client.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_embedding_client_new_stores_model_and_timeout() {
        let client = EmbeddingClient::new("http://localhost:8000", "bge-m3", 60);
        assert_eq!(client.model, "bge-m3");
        assert_eq!(client.timeout, Duration::from_secs(60));
    }
}
