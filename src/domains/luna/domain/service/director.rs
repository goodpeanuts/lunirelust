use crate::{
    common::error::AppError,
    domains::luna::dto::{
        CreateDirectorDto, DirectorDto, EntityCountDto, PaginatedResponse, PaginationQuery,
        SearchDirectorDto, UpdateDirectorDto,
    },
};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Trait defining business operations for director management.
pub trait DirectorServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn DirectorServiceTrait>
    where
        Self: Sized;

    /// Retrieves a director by their unique identifier.
    async fn get_director_by_id(&self, id: i64) -> Result<DirectorDto, AppError>;

    /// Retrieves director list by condition
    async fn get_director_list(
        &self,
        search_dto: SearchDirectorDto,
    ) -> Result<Vec<DirectorDto>, AppError>;

    /// Retrieves director list with pagination
    async fn get_director_list_paginated(
        &self,
        search_dto: SearchDirectorDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<DirectorDto>, AppError>;

    /// Retrieves all directors.
    async fn get_directors(&self) -> Result<Vec<DirectorDto>, AppError>;

    /// Creates a new director.
    async fn create_director(&self, create_dto: CreateDirectorDto)
        -> Result<DirectorDto, AppError>;

    /// Updates an existing director.
    async fn update_director(
        &self,
        id: i64,
        payload: UpdateDirectorDto,
    ) -> Result<DirectorDto, AppError>;

    /// Deletes a director by their unique identifier.
    async fn delete_director(&self, id: i64) -> Result<String, AppError>;

    /// Gets record counts grouped by directors.
    async fn get_director_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError>;
}
