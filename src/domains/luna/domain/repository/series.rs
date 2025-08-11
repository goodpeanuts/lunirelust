use crate::domains::luna::{
    domain::Series,
    dto::{CreateSeriesDto, EntityCountDto, SearchSeriesDto, UpdateSeriesDto},
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr};

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

    /// Gets record counts grouped by series.
    async fn get_series_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr>;
}
