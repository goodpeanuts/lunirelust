use crate::{
    common::error::AppError,
    common::pagination::paginate,
    domains::luna::{
        domain::{RecordRepository, RecordServiceTrait},
        dto::{
            CreateLinkDto, CreateRecordDto, PaginatedResponse, PaginationQuery, RecordDto,
            RecordSlimDto, SearchRecordDto, UpdateRecordDto,
        },
        infra::RecordRepo,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling record-related operations.
#[derive(Clone)]
pub struct RecordService {
    db: DatabaseConnection,
    repo: Arc<dyn RecordRepository + Send + Sync>,
}

#[async_trait]
impl RecordServiceTrait for RecordService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn RecordServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(RecordRepo),
        })
    }

    async fn get_record_by_id(&self, id: &str) -> Result<RecordDto, AppError> {
        let record = self
            .repo
            .find_by_id(&self.db, id.to_owned())
            .await
            .map_err(AppError::DatabaseError)?;

        record
            .map(RecordDto::from)
            .ok_or_else(|| AppError::NotFound("Record not found".into()))
    }

    async fn get_record_list(
        &self,
        search_dto: SearchRecordDto,
    ) -> Result<Vec<RecordDto>, AppError> {
        let records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(records.into_iter().map(RecordDto::from).collect())
    }

    async fn get_record_list_paginated(
        &self,
        search_dto: SearchRecordDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(paginate(records, &pagination, RecordDto::from))
    }

    async fn get_records(&self) -> Result<Vec<RecordDto>, AppError> {
        let records = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(records.into_iter().map(RecordDto::from).collect())
    }

    async fn get_all_record_ids(&self) -> Result<Vec<String>, AppError> {
        let ids = self
            .repo
            .find_all_ids(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(ids)
    }

    async fn get_all_record_slim(&self) -> Result<Vec<RecordSlimDto>, AppError> {
        let records = self
            .repo
            .find_all_slim(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(records.into_iter().map(RecordSlimDto::from).collect())
    }

    async fn create_record(&self, create_dto: CreateRecordDto) -> Result<RecordDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let id = match self.repo.create(&txn, create_dto).await {
            Ok(id) => id,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_record_by_id(&id).await
    }

    async fn update_record(
        &self,
        id: &str,
        update_dto: UpdateRecordDto,
    ) -> Result<RecordDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let updated_record = match self.repo.update(&txn, id.to_owned(), update_dto).await {
            Ok(r) => r,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        txn.commit().await.map_err(AppError::DatabaseError)?;

        updated_record
            .map(RecordDto::from)
            .ok_or_else(|| AppError::NotFound("Record not found".into()))
    }

    async fn update_record_links(
        &self,
        id: &str,
        new_links: Vec<CreateLinkDto>,
    ) -> Result<i32, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let result = match self
            .repo
            .update_record_links(&txn, id.to_owned(), new_links)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        txn.commit().await.map_err(AppError::DatabaseError)?;
        Ok(result)
    }

    async fn delete_record(&self, id: &str) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let deleted = match self.repo.delete(&txn, id.to_owned()).await {
            Ok(d) => d,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        if deleted {
            txn.commit().await.map_err(AppError::DatabaseError)?;
            Ok("Record deleted successfully".to_owned())
        } else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            Err(AppError::NotFound("Record not found".into()))
        }
    }

    async fn get_records_by_director(
        &self,
        director_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            director_id: Some(director_id),
            ..Default::default()
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(paginate(all_records, &pagination, RecordDto::from))
    }

    async fn get_records_by_studio(
        &self,
        studio_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            studio_id: Some(studio_id),
            ..Default::default()
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(paginate(all_records, &pagination, RecordDto::from))
    }

    async fn get_records_by_label(
        &self,
        label_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            label_id: Some(label_id),
            ..Default::default()
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(paginate(all_records, &pagination, RecordDto::from))
    }

    async fn get_records_by_series(
        &self,
        series_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            series_id: Some(series_id),
            ..Default::default()
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(paginate(all_records, &pagination, RecordDto::from))
    }

    async fn get_records_by_genre(
        &self,
        genre_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let all_records = self
            .repo
            .find_by_genre_id(&self.db, genre_id)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(paginate(all_records, &pagination, RecordDto::from))
    }

    async fn get_records_by_idol(
        &self,
        idol_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let all_records = self
            .repo
            .find_by_idol_id(&self.db, idol_id)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(paginate(all_records, &pagination, RecordDto::from))
    }
}
