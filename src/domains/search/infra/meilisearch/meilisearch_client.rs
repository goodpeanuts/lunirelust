//! `MeiliSearch` client wrapper.

use std::sync::Arc;

/// Wrapper around the `MeiliSearch` SDK client.
#[derive(Clone)]
pub struct MeiliSearchClient {
    pub client: Arc<meilisearch_sdk::client::Client>,
    pub index_name: String,
}

impl MeiliSearchClient {
    pub fn new(url: &str, master_key: &str, index_name: &str) -> Self {
        let client = meilisearch_sdk::client::Client::new(url, Some(master_key))
            .expect("Failed to create MeiliSearch client");
        Self {
            client: Arc::new(client),
            index_name: index_name.to_owned(),
        }
    }

    pub async fn health_check(&self) -> bool {
        match self.client.health().await {
            Ok(health) => health.status == "available",
            Err(e) => {
                tracing::warn!("MeiliSearch health check failed: {}", e);
                false
            }
        }
    }

    pub fn index(&self) -> meilisearch_sdk::indexes::Index {
        self.client.index(&self.index_name)
    }
}
