use crate::common::error::AppError;

use super::super::model::user_interaction::InteractionStatus;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
/// Trait defining business operations for user-record interactions.
pub trait InteractionServiceTrait: Send + Sync {
    /// Constructor for the interaction service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn InteractionServiceTrait>
    where
        Self: Sized;

    /// Toggle like status for a record. Returns the new liked state.
    async fn toggle_like(&self, user_id: &str, record_id: &str) -> Result<bool, AppError>;

    /// Mark a record as viewed by the user.
    async fn mark_viewed(&self, user_id: &str, record_id: &str) -> Result<(), AppError>;

    /// Batch-fetch interaction status for multiple records.
    async fn batch_get_status(
        &self,
        user_id: &str,
        record_ids: &[String],
    ) -> Result<HashMap<String, InteractionStatus>, AppError>;
}
