use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{RecordRepository, RecordServiceTrait},
        dto::{
            CreateLinkDto, CreateRecordDto, PaginatedResponse, PaginationQuery, RecordDto,
            RecordSlimDto, SearchRecordDto, UpdateRecordDto,
        },
        infra::RecordRepo,
    },
    domains::search::{
        OutboxRepo, OutboxRepository as _, SearchEntityType, TombstoneRepo,
        TombstoneRepository as _,
    },
};
use async_trait::async_trait;
use chrono::Utc;
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
        let paginated = self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(PaginatedResponse {
            count: paginated.count,
            next: paginated.next,
            previous: paginated.previous,
            results: paginated.results.into_iter().map(RecordDto::from).collect(),
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

        let (id, nested) = match self.repo.create(&txn, create_dto).await {
            Ok(result) => result,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        // Insert outbox events for nested named entities (version=0 for fan-out semantics)
        for (entity_type, entity_info) in [
            (SearchEntityType::Director, &nested.director),
            (SearchEntityType::Studio, &nested.studio),
            (SearchEntityType::Label, &nested.label),
            (SearchEntityType::Series, &nested.series),
        ] {
            if let Some((entity_id, entity_name)) = entity_info {
                if let Err(e) = crate::domains::luna::infra::search_outbox::outbox_entity_upsert(
                    &txn,
                    entity_type,
                    *entity_id,
                    entity_name,
                    vec![],
                )
                .await
                {
                    txn.rollback().await.ok();
                    return Err(AppError::DatabaseError(e));
                }
            }
        }

        for (genre_id, genre_name) in &nested.genres {
            if let Err(e) = crate::domains::luna::infra::search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Genre,
                *genre_id,
                genre_name,
                vec![],
            )
            .await
            {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        }

        for (idol_id, idol_name) in &nested.idols {
            if let Err(e) = crate::domains::luna::infra::search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Idol,
                *idol_id,
                idol_name,
                vec![],
            )
            .await
            {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        }

        // Insert outbox event + tombstone for the record itself
        let version = Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_millis() * 1_000_000);
        if let Err(e) = OutboxRepo::insert_event(
            &txn,
            SearchEntityType::Record.as_str(),
            &id,
            "upsert",
            version,
            None,
            None,
        )
        .await
        {
            txn.rollback().await.ok();
            return Err(AppError::DatabaseError(e));
        }
        if let Err(e) =
            TombstoneRepo::upsert_version(&txn, SearchEntityType::Record.as_str(), &id, version)
                .await
        {
            txn.rollback().await.ok();
            return Err(AppError::DatabaseError(e));
        }

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

        let Some(record) = updated_record else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::NotFound("Record not found".into()));
        };

        // Insert outbox event + tombstone within same transaction
        let version = Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_millis() * 1_000_000);
        if let Err(e) = OutboxRepo::insert_event(
            &txn,
            SearchEntityType::Record.as_str(),
            id,
            "upsert",
            version,
            None,
            None,
        )
        .await
        {
            txn.rollback().await.ok();
            return Err(AppError::DatabaseError(e));
        }
        if let Err(e) =
            TombstoneRepo::upsert_version(&txn, SearchEntityType::Record.as_str(), id, version)
                .await
        {
            txn.rollback().await.ok();
            return Err(AppError::DatabaseError(e));
        }

        txn.commit().await.map_err(AppError::DatabaseError)?;

        Ok(RecordDto::from(record))
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

        if !deleted {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::NotFound("Record not found".into()));
        }

        // Insert outbox delete event + mark tombstone within same transaction
        let version = Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_millis() * 1_000_000);
        if let Err(e) = OutboxRepo::insert_event(
            &txn,
            SearchEntityType::Record.as_str(),
            id,
            "delete",
            version,
            None,
            None,
        )
        .await
        {
            txn.rollback().await.ok();
            return Err(AppError::DatabaseError(e));
        }
        if let Err(e) =
            TombstoneRepo::mark_deleted(&txn, SearchEntityType::Record.as_str(), id, version).await
        {
            txn.rollback().await.ok();
            return Err(AppError::DatabaseError(e));
        }

        txn.commit().await.map_err(AppError::DatabaseError)?;
        Ok("Record deleted successfully".to_owned())
    }

    async fn get_records_by_director(
        &self,
        director_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        self.query_by_search_dto(
            SearchRecordDto {
                director_id: Some(director_id),
                ..Default::default()
            },
            pagination,
        )
        .await
    }

    async fn get_records_by_studio(
        &self,
        studio_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        self.query_by_search_dto(
            SearchRecordDto {
                studio_id: Some(studio_id),
                ..Default::default()
            },
            pagination,
        )
        .await
    }

    async fn get_records_by_label(
        &self,
        label_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        self.query_by_search_dto(
            SearchRecordDto {
                label_id: Some(label_id),
                ..Default::default()
            },
            pagination,
        )
        .await
    }

    async fn get_records_by_series(
        &self,
        series_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        self.query_by_search_dto(
            SearchRecordDto {
                series_id: Some(series_id),
                ..Default::default()
            },
            pagination,
        )
        .await
    }

    async fn get_records_by_genre(
        &self,
        genre_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let paginated = self
            .repo
            .find_by_genre_id_paginated(&self.db, genre_id, pagination)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(Self::to_paginated_response(paginated))
    }

    async fn get_records_by_idol(
        &self,
        idol_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let paginated = self
            .repo
            .find_by_idol_id_paginated(&self.db, idol_id, pagination)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(Self::to_paginated_response(paginated))
    }
}

impl RecordService {
    /// Query records using a `SearchRecordDto` filter with pagination.
    async fn query_by_search_dto(
        &self,
        search_dto: SearchRecordDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let paginated = self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(Self::to_paginated_response(paginated))
    }

    /// Convert an internal paginated result into the API response type.
    fn to_paginated_response(
        paginated: PaginatedResponse<crate::domains::luna::domain::Record>,
    ) -> PaginatedResponse<RecordDto> {
        PaginatedResponse {
            count: paginated.count,
            next: paginated.next,
            previous: paginated.previous,
            results: paginated.results.into_iter().map(RecordDto::from).collect(),
        }
    }
}
