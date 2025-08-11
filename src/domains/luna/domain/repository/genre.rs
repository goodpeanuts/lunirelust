use crate::domains::luna::{
    domain::Genre,
    dto::{
        CreateGenreDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchGenreDto,
        UpdateGenreDto,
    },
};

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

#[async_trait]
/// Trait representing repository-level operations for genre entities.
pub trait GenreRepository: Send + Sync {
    /// Retrieves all genres from the database.
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Genre>, DbErr>;

    /// Finds a genre by their unique identifier.
    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Genre>, DbErr>;

    /// Finds genre list by condition
    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchGenreDto,
    ) -> Result<Vec<Genre>, DbErr>;

    /// Finds genre list with pagination
    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchGenreDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Genre>, DbErr>;

    /// Creates a new genre record within an active transaction.
    async fn create(&self, txn: &DatabaseTransaction, genre: CreateGenreDto) -> Result<i64, DbErr>;

    /// Updates an existing genre record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        genre: UpdateGenreDto,
    ) -> Result<Option<Genre>, DbErr>;

    /// Deletes a genre by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr>;

    /// Gets record counts grouped by genres.
    async fn get_genre_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr>;
}
