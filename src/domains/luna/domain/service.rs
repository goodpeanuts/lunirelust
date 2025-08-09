//! This module defines service traits for luna (cards) domain entities,
//! responsible for business logic operations.

use crate::{
    common::error::AppError,
    domains::luna::dto::luna_dto::{
        CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateRecordDto,
        CreateSeriesDto, CreateStudioDto, DirectorDto, GenreDto, IdolDto, LabelDto,
        PaginatedResponse, PaginationQuery, RecordDto, SearchDirectorDto, SearchGenreDto,
        SearchIdolDto, SearchLabelDto, SearchRecordDto, SearchSeriesDto, SearchStudioDto,
        SeriesDto, StudioDto, UpdateDirectorDto, UpdateGenreDto, UpdateIdolDto, UpdateLabelDto,
        UpdateRecordDto, UpdateSeriesDto, UpdateStudioDto,
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
}

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
}

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
}

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
}

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
}

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
}

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

#[async_trait]
/// Combined service trait that includes all luna domain services.
pub trait LunaServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn LunaServiceTrait>
    where
        Self: Sized;

    /// Get director service
    fn director_service(&self) -> &dyn DirectorServiceTrait;

    /// Get genre service
    fn genre_service(&self) -> &dyn GenreServiceTrait;

    /// Get label service
    fn label_service(&self) -> &dyn LabelServiceTrait;

    /// Get studio service
    fn studio_service(&self) -> &dyn StudioServiceTrait;

    /// Get series service
    fn series_service(&self) -> &dyn SeriesServiceTrait;

    /// Get idol service
    fn idol_service(&self) -> &dyn IdolServiceTrait;

    /// Get record service
    fn record_service(&self) -> &dyn RecordServiceTrait;
}
