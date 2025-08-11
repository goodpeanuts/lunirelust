use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{SeriesRepository, SeriesServiceTrait},
        dto::{
            CreateSeriesDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchSeriesDto,
            SeriesDto, UpdateSeriesDto,
        },
        infra::SeriesRepo,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling series-related operations.
#[derive(Clone)]
pub struct SeriesService {
    db: DatabaseConnection,
    repo: Arc<dyn SeriesRepository + Send + Sync>,
}

#[async_trait]
impl SeriesServiceTrait for SeriesService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn SeriesServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(SeriesRepo),
        })
    }

    async fn get_series_by_id(&self, id: i64) -> Result<SeriesDto, AppError> {
        let series = self
            .repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?;

        series
            .map(SeriesDto::from)
            .ok_or_else(|| AppError::NotFound("Series not found".into()))
    }

    async fn get_series_list(
        &self,
        search_dto: SearchSeriesDto,
    ) -> Result<Vec<SeriesDto>, AppError> {
        let series = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(series.into_iter().map(SeriesDto::from).collect())
    }

    async fn get_series_list_paginated(
        &self,
        search_dto: SearchSeriesDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<SeriesDto>, AppError> {
        // Implementation similar to director service
        let series = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let limit = pagination.limit.unwrap_or(1000) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = series.len();
        let paginated_series: Vec<SeriesDto> = series
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(SeriesDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_series,
        })
    }

    async fn get_series(&self) -> Result<Vec<SeriesDto>, AppError> {
        let series = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(series.into_iter().map(SeriesDto::from).collect())
    }

    async fn create_series(&self, create_dto: CreateSeriesDto) -> Result<SeriesDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let id = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_series_by_id(id).await
    }

    async fn update_series(
        &self,
        id: i64,
        update_dto: UpdateSeriesDto,
    ) -> Result<SeriesDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let updated_series = self
            .repo
            .update(&txn, id, update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        updated_series
            .map(SeriesDto::from)
            .ok_or_else(|| AppError::NotFound("Series not found".into()))
    }

    async fn delete_series(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id)
            .await
            .map_err(AppError::DatabaseError)?;

        if deleted {
            txn.commit().await.map_err(AppError::DatabaseError)?;
            Ok("Series deleted successfully".to_owned())
        } else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            Err(AppError::NotFound("Series not found".into()))
        }
    }

    /// Gets record counts grouped by series.
    async fn get_series_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_series_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
