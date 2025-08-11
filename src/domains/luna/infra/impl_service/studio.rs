use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{StudioRepository, StudioServiceTrait},
        dto::{
            CreateStudioDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchStudioDto,
            StudioDto, UpdateStudioDto,
        },
        infra::StudioRepo,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling studio-related operations.
#[derive(Clone)]
pub struct StudioService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn StudioRepository + Send + Sync>,
}

#[async_trait]
impl StudioServiceTrait for StudioService {
    /// Creates a new studio service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn StudioServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(StudioRepo),
        })
    }

    /// Retrieves a studio by ID.
    async fn get_studio_by_id(&self, id: i64) -> Result<StudioDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(studio)) => Ok(StudioDto::from(studio)),
            Ok(None) => Err(AppError::NotFound("Studio not found".into())),
            Err(err) => {
                tracing::error!("Error finding studio by ID: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves studio list by condition.
    async fn get_studio_list(
        &self,
        search_dto: SearchStudioDto,
    ) -> Result<Vec<StudioDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(studios) => Ok(studios.into_iter().map(StudioDto::from).collect()),
            Err(err) => {
                tracing::error!("Error finding studio list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves studio list with pagination.
    async fn get_studio_list_paginated(
        &self,
        search_dto: SearchStudioDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<StudioDto>, AppError> {
        // For consistency, always use paginated approach
        let limit = pagination.limit.unwrap_or(20);
        let offset = pagination.offset.unwrap_or(0);
        let mut query = vec![];

        if let Some(id) = search_dto.id {
            query.push(("id", id.to_string()));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query.push(("name", name.to_owned()));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query.push(("link", link.to_owned()));
        }

        // Build query string
        let query_string = if query.is_empty() {
            String::new()
        } else {
            format!(
                "?{}",
                query
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("&")
            )
        };

        match self.repo.find_list(&self.db, search_dto).await {
            Ok(all_studios) => {
                let total_count = all_studios.len() as i64;

                // Apply manual pagination
                let studios: Vec<_> = all_studios
                    .into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect();

                // Calculate next and previous page URLs
                let base_url = format!("/cards/studios{query_string}");
                let has_next = (offset + limit) < total_count;
                let has_previous = offset > 0;

                let next = if has_next {
                    let sep = if query_string.is_empty() { "?" } else { "&" };
                    Some(format!(
                        "{}{}limit={}&offset={}",
                        base_url,
                        sep,
                        limit,
                        offset + limit
                    ))
                } else {
                    None
                };

                let previous = if has_previous {
                    let prev_offset = std::cmp::max(0, offset - limit);
                    let sep = if query_string.is_empty() { "?" } else { "&" };
                    Some(format!("{base_url}{sep}limit={limit}&offset={prev_offset}"))
                } else {
                    None
                };

                Ok(PaginatedResponse {
                    count: total_count,
                    next,
                    previous,
                    results: studios.into_iter().map(StudioDto::from).collect(),
                })
            }
            Err(err) => {
                tracing::error!("Error finding paginated studio list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all studios.
    async fn get_studios(&self) -> Result<Vec<StudioDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(studios) => Ok(studios.into_iter().map(StudioDto::from).collect()),
            Err(err) => {
                tracing::error!("Error finding all studios: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new studio.
    async fn create_studio(&self, create_dto: CreateStudioDto) -> Result<StudioDto, AppError> {
        let txn = match self.db.begin().await {
            Ok(txn) => txn,
            Err(err) => {
                tracing::error!("Error starting transaction: {err}");
                return Err(AppError::DatabaseError(err));
            }
        };

        match self.repo.create(&txn, create_dto).await {
            Ok(studio_id) => {
                if let Err(err) = txn.commit().await {
                    tracing::error!("Error committing transaction: {err}");
                    return Err(AppError::DatabaseError(err));
                }

                // Fetch the created studio
                self.get_studio_by_id(studio_id).await
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error creating studio: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing studio.
    async fn update_studio(
        &self,
        id: i64,
        update_dto: UpdateStudioDto,
    ) -> Result<StudioDto, AppError> {
        let txn = match self.db.begin().await {
            Ok(txn) => txn,
            Err(err) => {
                tracing::error!("Error starting transaction: {err}");
                return Err(AppError::DatabaseError(err));
            }
        };

        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(studio)) => {
                if let Err(err) = txn.commit().await {
                    tracing::error!("Error committing transaction: {err}");
                    return Err(AppError::DatabaseError(err));
                }
                Ok(StudioDto::from(studio))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Studio not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error updating studio: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a studio by their ID.
    async fn delete_studio(&self, id: i64) -> Result<String, AppError> {
        let txn = match self.db.begin().await {
            Ok(txn) => txn,
            Err(err) => {
                tracing::error!("Error starting transaction: {err}");
                return Err(AppError::DatabaseError(err));
            }
        };

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                if let Err(err) = txn.commit().await {
                    tracing::error!("Error committing transaction: {err}");
                    return Err(AppError::DatabaseError(err));
                }
                Ok("Studio deleted successfully".into())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Studio not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error deleting studio: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by studios.
    async fn get_studio_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_studio_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
