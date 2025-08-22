use crate::domains::luna::{
    domain::Record,
    dto::{CreateRecordDto, SearchRecordDto, UpdateRecordDto},
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

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

    /// Retrieves all record slim data from the database.
    async fn find_all_slim(&self, db: &DatabaseConnection) -> Result<Vec<Record>, DbErr>;
}
