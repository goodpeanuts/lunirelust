//! This module defines repository traits for luna (cards) domain entities,
//! which abstract the database operations.

use crate::domains::luna::dto::luna_dto::{
    CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateRecordDto,
    CreateSeriesDto, CreateStudioDto, PaginatedResponse, PaginationQuery, SearchDirectorDto,
    SearchGenreDto, SearchIdolDto, SearchLabelDto, SearchRecordDto, SearchSeriesDto,
    SearchStudioDto, UpdateDirectorDto, UpdateGenreDto, UpdateIdolDto, UpdateLabelDto,
    UpdateRecordDto, UpdateSeriesDto, UpdateStudioDto,
};

use super::model::{Director, Genre, Idol, Label, Record, Series, Studio};

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
}

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
}

#[async_trait]
/// Trait representing repository-level operations for idol entities.
pub trait IdolRepository: Send + Sync {
    /// Retrieves all idols from the database.
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Idol>, DbErr>;

    /// Finds an idol by their unique identifier.
    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Idol>, DbErr>;

    /// Finds idol list by condition with search support
    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchIdolDto,
    ) -> Result<Vec<Idol>, DbErr>;

    /// Creates a new idol record within an active transaction.
    async fn create(&self, txn: &DatabaseTransaction, idol: CreateIdolDto) -> Result<i64, DbErr>;

    /// Updates an existing idol record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        idol: UpdateIdolDto,
    ) -> Result<Option<Idol>, DbErr>;

    /// Deletes an idol by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr>;
}

#[async_trait]
/// Trait representing repository-level operations for label entities.
pub trait LabelRepository: Send + Sync {
    /// Retrieves all labels from the database.
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Label>, DbErr>;

    /// Finds a label by their unique identifier.
    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Label>, DbErr>;

    /// Finds label list by condition
    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchLabelDto,
    ) -> Result<Vec<Label>, DbErr>;

    /// Finds label list with pagination
    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchLabelDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Label>, DbErr>;

    /// Creates a new label record within an active transaction.
    async fn create(&self, txn: &DatabaseTransaction, label: CreateLabelDto) -> Result<i64, DbErr>;

    /// Updates an existing label record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        label: UpdateLabelDto,
    ) -> Result<Option<Label>, DbErr>;

    /// Deletes a label by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr>;
}

#[async_trait]
/// Trait representing repository-level operations for studio entities.
pub trait StudioRepository: Send + Sync {
    /// Retrieves all studios from the database.
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Studio>, DbErr>;

    /// Finds a studio by their unique identifier.
    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Studio>, DbErr>;

    /// Finds studio list by condition
    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchStudioDto,
    ) -> Result<Vec<Studio>, DbErr>;

    /// Creates a new studio record within an active transaction.
    async fn create(
        &self,
        txn: &DatabaseTransaction,
        studio: CreateStudioDto,
    ) -> Result<i64, DbErr>;

    /// Updates an existing studio record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        studio: UpdateStudioDto,
    ) -> Result<Option<Studio>, DbErr>;

    /// Deletes a studio by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr>;
}

#[async_trait]
/// Trait representing repository-level operations for series entities.
pub trait SeriesRepository: Send + Sync {
    /// Retrieves all series from the database.
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Series>, DbErr>;

    /// Finds a series by their unique identifier.
    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Series>, DbErr>;

    /// Finds series list by condition
    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchSeriesDto,
    ) -> Result<Vec<Series>, DbErr>;

    /// Creates a new series record within an active transaction.
    async fn create(
        &self,
        txn: &DatabaseTransaction,
        series: CreateSeriesDto,
    ) -> Result<i64, DbErr>;

    /// Updates an existing series record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        series: UpdateSeriesDto,
    ) -> Result<Option<Series>, DbErr>;

    /// Deletes a series by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr>;
}

#[async_trait]
/// Trait representing repository-level operations for record entities.
pub trait RecordRepository: Send + Sync {
    /// Retrieves all records from the database.
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Record>, DbErr>;

    /// Finds a record by their unique identifier.
    async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Option<Record>, DbErr>;

    /// Finds record list by condition with search support
    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchRecordDto,
    ) -> Result<Vec<Record>, DbErr>;

    /// Creates a new record within an active transaction.
    async fn create(
        &self,
        txn: &DatabaseTransaction,
        record: CreateRecordDto,
    ) -> Result<String, DbErr>;

    /// Updates an existing record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: String,
        record: UpdateRecordDto,
    ) -> Result<Option<Record>, DbErr>;

    /// Deletes a record by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: String) -> Result<bool, DbErr>;
}
