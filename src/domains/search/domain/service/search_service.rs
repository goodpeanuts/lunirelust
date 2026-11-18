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

    /// Return maximum search permission for an existing authenticated user.
    /// Missing users or lookup failures receive zero; per-user levels are not implemented.
    async fn get_user_permission(&self, user_id: &str) -> i32;

    /// Persist `PostgreSQL` mode before writes, or start enhanced repair in the background.
    async fn trigger_startup_sync(&self) -> Result<(), AppError>;
}
