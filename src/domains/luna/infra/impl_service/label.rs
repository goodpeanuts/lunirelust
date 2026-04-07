use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{LabelRepository, LabelServiceTrait},
        dto::{
            CreateLabelDto, EntityCountDto, LabelDto, PaginatedResponse, PaginationQuery,
            SearchLabelDto, UpdateLabelDto,
        },
        infra::LabelRepo,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling label-related operations.
#[derive(Clone)]
pub struct LabelService {
    db: DatabaseConnection,
    repo: Arc<dyn LabelRepository + Send + Sync>,
}

#[async_trait]
impl LabelServiceTrait for LabelService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn LabelServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(LabelRepo {}),
        })
    }

    async fn get_label_by_id(&self, id: i64) -> Result<LabelDto, AppError> {
        self.repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(LabelDto::from)
            .ok_or_else(|| AppError::NotFound("Label not found".into()))
    }

    async fn get_label_list(&self, search_dto: SearchLabelDto) -> Result<Vec<LabelDto>, AppError> {
        let labels = self.repo.find_list(&self.db, search_dto).await?;
        Ok(labels.into_iter().map(Into::into).collect())
    }

    async fn get_label_list_paginated(
        &self,
        search_dto: SearchLabelDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<LabelDto>, AppError> {
        let paginated = self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await?;
        Ok(PaginatedResponse {
            count: paginated.count,
            next: paginated.next,
            previous: paginated.previous,
            results: paginated.results.into_iter().map(Into::into).collect(),
        })
    }

    async fn get_labels(&self) -> Result<Vec<LabelDto>, AppError> {
        let labels = self.repo.find_all(&self.db).await?;
        Ok(labels.into_iter().map(Into::into).collect())
    }

    async fn create_label(&self, create_dto: CreateLabelDto) -> Result<LabelDto, AppError> {
        let txn = self.db.begin().await?;
        let label_id = match self.repo.create(&txn, create_dto).await {
            Ok(id) => id,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };
        txn.commit().await?;
        self.get_label_by_id(label_id).await
    }

    async fn update_label(
        &self,
        id: i64,
        update_dto: UpdateLabelDto,
    ) -> Result<LabelDto, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(label)) => {
                txn.commit().await?;
                Ok(LabelDto::from(label))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Label not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn delete_label(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await?;
                Ok("Label deleted successfully".to_owned())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Label not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn get_label_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_label_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
