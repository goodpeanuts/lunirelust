use crate::domains::luna::{
    domain::Label,
    dto::{
        CreateLabelDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchLabelDto,
        UpdateLabelDto,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

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

    /// Gets record counts grouped by labels.
    async fn get_label_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr>;
}
