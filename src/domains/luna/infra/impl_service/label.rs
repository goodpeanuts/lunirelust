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
    pub db: DatabaseConnection,
    pub repo: Arc<dyn LabelRepository + Send + Sync>,
}

#[async_trait]
impl LabelServiceTrait for LabelService {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn LabelServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(LabelRepo {}),
        })
    }

    /// Retrieves a label by their ID.
    async fn get_label_by_id(&self, id: i64) -> Result<LabelDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(label)) => Ok(LabelDto::from(label)),
            Ok(None) => Err(AppError::NotFound("Label not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves label list by condition
    async fn get_label_list(&self, search_dto: SearchLabelDto) -> Result<Vec<LabelDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(labels) => {
                let label_dtos: Vec<LabelDto> = labels.into_iter().map(Into::into).collect();
                Ok(label_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving label list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves label list with pagination
    async fn get_label_list_paginated(
        &self,
        search_dto: SearchLabelDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<LabelDto>, AppError> {
        match self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
        {
            Ok(paginated_response) => {
                let label_dtos: Vec<LabelDto> = paginated_response
                    .results
                    .into_iter()
                    .map(Into::into)
                    .collect();
                Ok(PaginatedResponse {
                    count: paginated_response.count,
                    next: paginated_response.next,
                    previous: paginated_response.previous,
                    results: label_dtos,
                })
            }
            Err(err) => {
                tracing::error!("Error retrieving paginated label list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all labels from the database.
    async fn get_labels(&self) -> Result<Vec<LabelDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(labels) => {
                let label_dtos: Vec<LabelDto> = labels.into_iter().map(Into::into).collect();
                Ok(label_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving all labels: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new label.
    async fn create_label(&self, create_dto: CreateLabelDto) -> Result<LabelDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.create(&txn, create_dto).await {
            Ok(label_id) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                self.get_label_by_id(label_id).await
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error creating label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing label.
    async fn update_label(
        &self,
        id: i64,
        update_dto: UpdateLabelDto,
    ) -> Result<LabelDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(label)) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok(LabelDto::from(label))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Label not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error updating label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a label by their ID.
    async fn delete_label(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok("Label deleted successfully".to_owned())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Label not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error deleting label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by labels.
    async fn get_label_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_label_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
