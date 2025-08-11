use crate::{
    common::error::AppError,
    domains::luna::dto::{
        CreateStudioDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchStudioDto,
        StudioDto, UpdateStudioDto,
    },
};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Service trait for studio-related business logic operations.
pub trait StudioServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn StudioServiceTrait>
    where
        Self: Sized;

    /// Retrieves a studio by their unique identifier.
    async fn get_studio_by_id(&self, id: i64) -> Result<StudioDto, AppError>;

    /// Retrieves studio list by condition
    async fn get_studio_list(
        &self,
        search_dto: SearchStudioDto,
    ) -> Result<Vec<StudioDto>, AppError>;

    /// Retrieves studio list with pagination
    async fn get_studio_list_paginated(
        &self,
        search_dto: SearchStudioDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<StudioDto>, AppError>;

    /// Retrieves all studios.
    async fn get_studios(&self) -> Result<Vec<StudioDto>, AppError>;

    /// Creates a new studio.
    async fn create_studio(&self, create_dto: CreateStudioDto) -> Result<StudioDto, AppError>;

    /// Updates an existing studio.
    async fn update_studio(
        &self,
        id: i64,
        update_dto: UpdateStudioDto,
    ) -> Result<StudioDto, AppError>;

    /// Deletes a studio by their unique identifier.
    async fn delete_studio(&self, id: i64) -> Result<String, AppError>;

    /// Gets record counts grouped by studios.
    async fn get_studio_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError>;
}
