use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{Record, RecordRepository, RecordServiceTrait},
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

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = records.len();
        let paginated_records: Vec<RecordDto> = records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
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
            results: paginated_records,
        })
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

        let id = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_record_by_id(&id).await
    }

    async fn update_record(
        &self,
        id: &str,
        update_dto: UpdateRecordDto,
    ) -> Result<RecordDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let updated_record = self
            .repo
            .update(&txn, id.to_owned(), update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

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

        let result = self
            .repo
            .update_record_links(&txn, id.to_owned(), new_links)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;
        Ok(result)
    }

    async fn delete_record(&self, id: &str) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id.to_owned())
            .await
            .map_err(AppError::DatabaseError)?;

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
        // Simplified implementation using search
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        // Filter by director
        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.director.id == director_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
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
            results: paginated_records,
        })
    }

    async fn get_records_by_studio(
        &self,
        studio_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        // Similar implementation to get_records_by_director
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.studio.id == studio_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
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
            results: paginated_records,
        })
    }

    async fn get_records_by_label(
        &self,
        label_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.label.id == label_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
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
            results: paginated_records,
        })
    }

    async fn get_records_by_series(
        &self,
        series_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.series.id == series_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
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
            results: paginated_records,
        })
    }

    async fn get_records_by_genre(
        &self,
        genre_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        // Filter by genre using the genres relation
        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.genres.iter().any(|rg| rg.genre.id == genre_id))
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
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
            results: paginated_records,
        })
    }

    async fn get_records_by_idol(
        &self,
        idol_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        // Filter by idol using the idols relation
        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.idols.iter().any(|ip| ip.idol.id == idol_id))
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
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
            results: paginated_records,
        })
    }
}
