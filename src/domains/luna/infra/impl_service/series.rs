use crate::domains::search::SearchEntityType;
use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{SeriesRepository, SeriesServiceTrait},
        dto::{
            CreateSeriesDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchSeriesDto,
            SeriesDto, UpdateSeriesDto,
        },
        infra::{search_outbox, SeriesRepo},
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
        let paginated = self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(PaginatedResponse {
            count: paginated.count,
            next: paginated.next,
            previous: paginated.previous,
            results: paginated.results.into_iter().map(SeriesDto::from).collect(),
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

        let entity_name = create_dto.name.clone();
        let (id, was_created) = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        if was_created {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Series,
                id,
                &entity_name,
                Vec::new(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_series_by_id(id).await
    }

    async fn update_series(
        &self,
        id: i64,
        update_dto: UpdateSeriesDto,
    ) -> Result<SeriesDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let pre_affected =
            search_outbox::find_affected_record_ids(&txn, SearchEntityType::Series, id)
                .await
                .map_err(AppError::DatabaseError)?;

        let updated_series = self
            .repo
            .update(&txn, id, update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let Some(series) = updated_series else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::NotFound("Series not found".into()));
        };

        let surviving_id = series.id;
        if surviving_id != id {
            search_outbox::outbox_entity_delete(&txn, SearchEntityType::Series, id, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
            let mut all_affected = pre_affected.clone();
            let surviving_affected = search_outbox::find_affected_record_ids(
                &txn,
                SearchEntityType::Series,
                surviving_id,
            )
            .await
            .map_err(AppError::DatabaseError)?;
            all_affected.extend(surviving_affected);
            all_affected.sort_unstable();
            all_affected.dedup();
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Series,
                surviving_id,
                &series.name,
                all_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &all_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        } else {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Series,
                id,
                &series.name,
                pre_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &pre_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await.map_err(AppError::DatabaseError)?;

        Ok(SeriesDto::from(series))
    }

    async fn delete_series(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let affected = search_outbox::find_affected_record_ids(&txn, SearchEntityType::Series, id)
            .await
            .map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id)
            .await
            .map_err(AppError::DatabaseError)?;

        if !deleted {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::NotFound("Series not found".into()));
        }

        search_outbox::outbox_entity_delete(&txn, SearchEntityType::Series, id, affected.clone())
            .await
            .map_err(AppError::DatabaseError)?;
        search_outbox::outbox_fanout_records(&txn, &affected)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;
        Ok("Series deleted successfully".to_owned())
    }

    /// Gets record counts grouped by series.
    async fn get_series_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_series_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
