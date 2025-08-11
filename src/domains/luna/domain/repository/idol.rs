use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

use crate::domains::luna::{
    domain::Idol,
    dto::{CreateIdolDto, EntityCountDto, SearchIdolDto, UpdateIdolDto},
};

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

    /// Gets record counts grouped by idols.
    async fn get_idol_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr>;
}
