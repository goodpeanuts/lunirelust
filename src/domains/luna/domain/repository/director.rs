use crate::domains::luna::{
    domain::Director,
    dto::{
        CreateDirectorDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchDirectorDto,
        UpdateDirectorDto,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

#[async_trait]
/// Trait representing repository-level operations for director entities.
pub trait DirectorRepository: Send + Sync {
    /// Retrieves all directors from the database.
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Director>, DbErr>;

    /// Finds a director by their unique identifier.
    async fn find_by_id(&self, db: &DatabaseConnection, id: i64)
        -> Result<Option<Director>, DbErr>;

    /// Finds director list by condition
    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchDirectorDto,
    ) -> Result<Vec<Director>, DbErr>;

    /// Finds director list with pagination
    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchDirectorDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Director>, DbErr>;

    /// Creates a new director record within an active transaction.
    async fn create(
        &self,
        txn: &DatabaseTransaction,
        director: CreateDirectorDto,
    ) -> Result<i64, DbErr>;

    /// Updates an existing director record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        director: UpdateDirectorDto,
    ) -> Result<Option<Director>, DbErr>;

    /// Deletes a director by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr>;

    /// Gets record counts grouped by directors.
    async fn get_director_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr>;
}
