//! `SearchService` trait definition.

use crate::common::config::Config;
use crate::common::error::AppError;
use crate::domains::search::dto::{SearchQuery, SearchResponse};
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Trait for search service operations.
#[async_trait]
pub trait SearchServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(config: Config, db: DatabaseConnection) -> Arc<dyn SearchServiceTrait>
    where
        Self: Sized;

    /// Execute a search query and return results.
    async fn search(
        &self,
        query: SearchQuery,
        user_permission: i32,
    ) -> Result<SearchResponse, AppError>;

    /// Check if `MeiliSearch` is ready to serve queries.
    fn is_meili_ready(&self) -> bool;

    /// Look up the caller's permission level from `user_ext`.
    /// Falls back to 0 (no access to restricted records) on lookup failure.
    async fn get_user_permission(&self, user_id: &str) -> i32;

    /// Trigger startup full sync (runs in background).
    fn trigger_startup_sync(&self);
}
