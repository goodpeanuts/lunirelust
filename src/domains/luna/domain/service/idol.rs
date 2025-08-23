use crate::{
    common::error::AppError,
    domains::luna::dto::{
        CreateIdolDto, EntityCountDto, IdolDto, IdolWithoutImageDto, PaginatedResponse,
        PaginationQuery, SearchIdolDto, UpdateIdolDto,
    },
};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Service trait for idol-related business logic operations.
pub trait IdolServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn IdolServiceTrait>
    where
        Self: Sized;

    /// Retrieves an idol by their unique identifier.
    async fn get_idol_by_id(&self, id: i64) -> Result<IdolDto, AppError>;

    /// Retrieves idol list by condition
    async fn get_idol_list(&self, search_dto: SearchIdolDto) -> Result<Vec<IdolDto>, AppError>;

    /// Retrieves idol list with pagination
    async fn get_idol_list_paginated(
        &self,
        search_dto: SearchIdolDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<IdolDto>, AppError>;

    /// Retrieves all idols.
    async fn get_idols(&self) -> Result<Vec<IdolDto>, AppError>;

    /// Creates a new idol.
    async fn create_idol(&self, create_dto: CreateIdolDto) -> Result<IdolDto, AppError>;

    /// Updates an existing idol.
    async fn update_idol(&self, id: i64, update_dto: UpdateIdolDto) -> Result<IdolDto, AppError>;

    /// Deletes an idol by their unique identifier.
    async fn delete_idol(&self, id: i64) -> Result<String, AppError>;

    /// Gets record counts grouped by idols.
    async fn get_idol_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError>;

    /// Gets idols that don't have any images in the media directory.
    async fn get_idols_without_images(
        &self,
        assets_private_path: &str,
    ) -> Result<Vec<IdolWithoutImageDto>, AppError>;
}
