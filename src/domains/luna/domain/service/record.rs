use crate::{
    common::error::AppError,
    domains::luna::dto::{
        CreateRecordDto, PaginatedResponse, PaginationQuery, RecordDto, SearchRecordDto,
        UpdateRecordDto,
    },
};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Service trait for record-related business logic operations.
pub trait RecordServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn RecordServiceTrait>
    where
        Self: Sized;

    /// Retrieves a record by their unique identifier.
    async fn get_record_by_id(&self, id: &str) -> Result<RecordDto, AppError>;

    /// Retrieves record list by condition
    async fn get_record_list(
        &self,
        search_dto: SearchRecordDto,
    ) -> Result<Vec<RecordDto>, AppError>;

    /// Retrieves record list with pagination
    async fn get_record_list_paginated(
        &self,
        search_dto: SearchRecordDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError>;

    /// Retrieves all records.
    async fn get_records(&self) -> Result<Vec<RecordDto>, AppError>;

    /// Creates a new record.
    async fn create_record(&self, create_dto: CreateRecordDto) -> Result<RecordDto, AppError>;

    /// Updates an existing record.
    async fn update_record(
        &self,
        id: &str,
        update_dto: UpdateRecordDto,
    ) -> Result<RecordDto, AppError>;

    /// Deletes a record by their unique identifier.
    async fn delete_record(&self, id: &str) -> Result<String, AppError>;

    /// Get records by director ID with pagination
    async fn get_records_by_director(
        &self,
        director_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError>;

    /// Get records by studio ID with pagination
    async fn get_records_by_studio(
        &self,
        studio_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError>;

    /// Get records by label ID with pagination
    async fn get_records_by_label(
        &self,
        label_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError>;

    /// Get records by series ID with pagination
    async fn get_records_by_series(
        &self,
        series_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError>;

    /// Get records by genre ID with pagination
    async fn get_records_by_genre(
        &self,
        genre_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError>;

    /// Get records by idol ID with pagination
    async fn get_records_by_idol(
        &self,
        idol_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError>;
}
