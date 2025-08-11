use crate::{
    common::error::AppError,
    domains::luna::dto::{
        CreateGenreDto, EntityCountDto, GenreDto, PaginatedResponse, PaginationQuery,
        SearchGenreDto, UpdateGenreDto,
    },
};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Trait defining business operations for genre management.
pub trait GenreServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn GenreServiceTrait>
    where
        Self: Sized;

    /// Retrieves a genre by their unique identifier.
    async fn get_genre_by_id(&self, id: i64) -> Result<GenreDto, AppError>;

    /// Retrieves genre list by condition
    async fn get_genre_list(&self, search_dto: SearchGenreDto) -> Result<Vec<GenreDto>, AppError>;

    /// Retrieves genre list with pagination
    async fn get_genre_list_paginated(
        &self,
        search_dto: SearchGenreDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<GenreDto>, AppError>;

    /// Retrieves all genres.
    async fn get_genres(&self) -> Result<Vec<GenreDto>, AppError>;

    /// Creates a new genre.
    async fn create_genre(&self, create_dto: CreateGenreDto) -> Result<GenreDto, AppError>;

    /// Updates an existing genre.
    async fn update_genre(&self, id: i64, payload: UpdateGenreDto) -> Result<GenreDto, AppError>;

    /// Deletes a genre by their unique identifier.
    async fn delete_genre(&self, id: i64) -> Result<String, AppError>;

    /// Gets record counts grouped by genres.
    async fn get_genre_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError>;
}
