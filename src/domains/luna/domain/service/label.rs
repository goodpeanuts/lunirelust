use crate::{
    common::error::AppError,
    domains::luna::dto::{
        CreateLabelDto, EntityCountDto, LabelDto, PaginatedResponse, PaginationQuery,
        SearchLabelDto, UpdateLabelDto,
    },
};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Trait defining business operations for label management.
pub trait LabelServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn LabelServiceTrait>
    where
        Self: Sized;

    /// Retrieves a label by their unique identifier.
    async fn get_label_by_id(&self, id: i64) -> Result<LabelDto, AppError>;

    /// Retrieves label list by condition
    async fn get_label_list(&self, search_dto: SearchLabelDto) -> Result<Vec<LabelDto>, AppError>;

    /// Retrieves label list with pagination
    async fn get_label_list_paginated(
        &self,
        search_dto: SearchLabelDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<LabelDto>, AppError>;

    /// Retrieves all labels.
    async fn get_labels(&self) -> Result<Vec<LabelDto>, AppError>;

    /// Creates a new label.
    async fn create_label(&self, create_dto: CreateLabelDto) -> Result<LabelDto, AppError>;

    /// Updates an existing label.
    async fn update_label(&self, id: i64, payload: UpdateLabelDto) -> Result<LabelDto, AppError>;

    /// Deletes a label by their unique identifier.
    async fn delete_label(&self, id: i64) -> Result<String, AppError>;

    /// Gets record counts grouped by labels.
    async fn get_label_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError>;
}
