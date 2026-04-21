use crate::domains::user::domain::model::user_interaction::InteractionStatus;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr};

#[async_trait]
/// Trait representing repository-level operations for user-record interactions.
pub trait InteractionRepository: Send + Sync {
    /// Toggle like status for a user-record pair.
    /// If no row exists, creates one with liked=true.
    /// If liked=true, sets liked=false and clears `liked_at`.
    /// If liked=false, sets liked=true and sets `liked_at` to now.
    async fn toggle_like(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        record_id: &str,
    ) -> Result<bool, DbErr>;

    /// Mark a record as viewed by the user.
    /// Creates the row if it does not exist.
    async fn mark_viewed(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        record_id: &str,
    ) -> Result<(), DbErr>;

    /// Batch-fetch interaction status for multiple records.
    /// Returns a map of `record_id` -> `InteractionStatus`.
    async fn batch_get_status(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        record_ids: &[String],
    ) -> Result<std::collections::HashMap<String, InteractionStatus>, DbErr>;

    /// Retrieves paginated record IDs that the user has viewed.
    /// Returns (IDs, total count).
    async fn find_viewed_record_ids_paginated(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(Vec<String>, u64), DbErr>;
}
