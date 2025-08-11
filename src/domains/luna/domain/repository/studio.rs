use crate::domains::luna::{
    domain::Studio,
    dto::{CreateStudioDto, EntityCountDto, SearchStudioDto, UpdateStudioDto},
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

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

    /// Gets record counts grouped by studios.
    async fn get_studio_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr>;
}
