use crate::{
    common::error::AppError,
    domains::luna::dto::{
        CreateSeriesDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchSeriesDto,
        SeriesDto, UpdateSeriesDto,
    },
};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Service trait for series-related business logic operations.
pub trait SeriesServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn SeriesServiceTrait>
    where
        Self: Sized;

    /// Retrieves a series by their unique identifier.
    async fn get_series_by_id(&self, id: i64) -> Result<SeriesDto, AppError>;

    /// Retrieves series list by condition
    async fn get_series_list(
        &self,
        search_dto: SearchSeriesDto,
    ) -> Result<Vec<SeriesDto>, AppError>;

    /// Retrieves series list with pagination
    async fn get_series_list_paginated(
        &self,
        search_dto: SearchSeriesDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<SeriesDto>, AppError>;

    /// Retrieves all series.
    async fn get_series(&self) -> Result<Vec<SeriesDto>, AppError>;

    /// Creates a new series.
    async fn create_series(&self, create_dto: CreateSeriesDto) -> Result<SeriesDto, AppError>;

    /// Updates an existing series.
    async fn update_series(
        &self,
        id: i64,
        update_dto: UpdateSeriesDto,
    ) -> Result<SeriesDto, AppError>;

    /// Deletes a series by their unique identifier.
    async fn delete_series(&self, id: i64) -> Result<String, AppError>;

    /// Gets record counts grouped by series.
    async fn get_series_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError>;
}
