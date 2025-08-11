use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{IdolRepository, IdolServiceTrait},
        dto::{
            CreateIdolDto, EntityCountDto, IdolDto, PaginatedResponse, PaginationQuery,
            SearchIdolDto, UpdateIdolDto,
        },
        infra::IdolRepo,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling idol-related operations.
#[derive(Clone)]
pub struct IdolService {
    db: DatabaseConnection,
    repo: Arc<dyn IdolRepository + Send + Sync>,
}

#[async_trait]
impl IdolServiceTrait for IdolService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn IdolServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(IdolRepo),
        })
    }

    async fn get_idol_by_id(&self, id: i64) -> Result<IdolDto, AppError> {
        let idol = self
            .repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?;

        idol.map(IdolDto::from)
            .ok_or_else(|| AppError::NotFound("Idol not found".into()))
    }

    async fn get_idol_list(&self, search_dto: SearchIdolDto) -> Result<Vec<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(idols.into_iter().map(IdolDto::from).collect())
    }

    async fn get_idol_list_paginated(
        &self,
        search_dto: SearchIdolDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let limit = pagination.limit.unwrap_or(1000) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = idols.len();
        let paginated_idols: Vec<IdolDto> = idols
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(IdolDto::from)
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
            results: paginated_idols,
        })
    }

    async fn get_idols(&self) -> Result<Vec<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(idols.into_iter().map(IdolDto::from).collect())
    }

    async fn create_idol(&self, create_dto: CreateIdolDto) -> Result<IdolDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let id = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_idol_by_id(id).await
    }

    async fn update_idol(&self, id: i64, update_dto: UpdateIdolDto) -> Result<IdolDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let updated_idol = self
            .repo
            .update(&txn, id, update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        updated_idol
            .map(IdolDto::from)
            .ok_or_else(|| AppError::NotFound("Idol not found".into()))
    }

    async fn delete_idol(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id)
            .await
            .map_err(AppError::DatabaseError)?;

        if deleted {
            txn.commit().await.map_err(AppError::DatabaseError)?;
            Ok("Idol deleted successfully".to_owned())
        } else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            Err(AppError::NotFound("Idol not found".into()))
        }
    }

    /// Gets record counts grouped by idols.
    async fn get_idol_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_idol_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
