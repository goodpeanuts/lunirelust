use crate::domains::luna::{
    domain::Record,
    dto::{
        CreateLinkDto, CreateRecordDto, PaginatedResponse, PaginationQuery, SearchRecordDto,
        UpdateRecordDto, UserFilter,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

/// Tracks nested named entities created during a record creation.
#[derive(Debug, Default)]
pub struct CreatedNestedEntities {
    pub director: Option<(i64, String)>,
    pub studio: Option<(i64, String)>,
    pub label: Option<(i64, String)>,
    pub series: Option<(i64, String)>,
    pub genres: Vec<(i64, String)>,
    pub idols: Vec<(i64, String)>,
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

    /// Finds record list with database-level pagination using LIMIT/OFFSET.
    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchRecordDto,
        pagination: PaginationQuery,
        user_filter: Option<UserFilter>,
    ) -> Result<PaginatedResponse<Record>, DbErr>;

    /// Creates a new record within an active transaction.
    /// Returns the record ID and info about any nested named entities created.
    async fn create(
        &self,
        txn: &DatabaseTransaction,
        record: CreateRecordDto,
    ) -> Result<(String, CreatedNestedEntities), DbErr>;

    /// Updates an existing record.
    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: String,
        record: UpdateRecordDto,
    ) -> Result<Option<Record>, DbErr>;

    /// Deletes a record by their unique identifier within an active transaction.
    async fn delete(&self, txn: &DatabaseTransaction, id: String) -> Result<bool, DbErr>;

    /// Update record links only - add new links that don't already exist
    /// Returns the number of new links added
    async fn update_record_links(
        &self,
        txn: &DatabaseTransaction,
        record_id: String,
        new_links: Vec<CreateLinkDto>,
    ) -> Result<i32, DbErr>;

    /// Retrieves all record slim data from the database.
    async fn find_all_slim(&self, db: &DatabaseConnection) -> Result<Vec<Record>, DbErr>;

    /// Retrieves all record IDs from the database.
    async fn find_all_ids(&self, db: &DatabaseConnection) -> Result<Vec<String>, DbErr>;

    /// Finds records filtered by genre via JOIN on `record_genre` table.
    #[expect(dead_code)]
    async fn find_by_genre_id(
        &self,
        db: &DatabaseConnection,
        genre_id: i64,
    ) -> Result<Vec<Record>, DbErr>;

    /// Finds records filtered by genre with database-level pagination.
    async fn find_by_genre_id_paginated(
        &self,
        db: &DatabaseConnection,
        genre_id: i64,
        pagination: PaginationQuery,
        user_filter: Option<UserFilter>,
    ) -> Result<PaginatedResponse<Record>, DbErr>;

    /// Finds records filtered by idol via JOIN on `idol_participation` table.
    #[expect(dead_code)]
    async fn find_by_idol_id(
        &self,
        db: &DatabaseConnection,
        idol_id: i64,
    ) -> Result<Vec<Record>, DbErr>;

    /// Finds records filtered by idol with database-level pagination.
    async fn find_by_idol_id_paginated(
        &self,
        db: &DatabaseConnection,
        idol_id: i64,
        pagination: PaginationQuery,
        user_filter: Option<UserFilter>,
    ) -> Result<PaginatedResponse<Record>, DbErr>;
}
