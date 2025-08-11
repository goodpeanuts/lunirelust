use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{DirectorRepository, DirectorServiceTrait},
        dto::{
            CreateDirectorDto, DirectorDto, EntityCountDto, PaginatedResponse, PaginationQuery,
            SearchDirectorDto, UpdateDirectorDto,
        },
        infra::DirectorRepo,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling director-related operations.
#[derive(Clone)]
pub struct DirectorService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn DirectorRepository + Send + Sync>,
}

#[async_trait]
impl DirectorServiceTrait for DirectorService {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn DirectorServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(DirectorRepo {}),
        })
    }

    /// Retrieves a director by their ID.
    async fn get_director_by_id(&self, id: i64) -> Result<DirectorDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(director)) => Ok(DirectorDto::from(director)),
            Ok(None) => Err(AppError::NotFound("Director not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving director: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves director list by condition
    async fn get_director_list(
        &self,
        search_dto: SearchDirectorDto,
    ) -> Result<Vec<DirectorDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(directors) => {
                let director_dtos: Vec<DirectorDto> =
                    directors.into_iter().map(Into::into).collect();
                Ok(director_dtos)
            }
            Err(err) => {
                tracing::error!("Error fetching directors: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves director list with pagination
    async fn get_director_list_paginated(
        &self,
        search_dto: SearchDirectorDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<DirectorDto>, AppError> {
        match self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
        {
            Ok(paginated_response) => {
                let director_dtos: Vec<DirectorDto> = paginated_response
                    .results
                    .into_iter()
                    .map(Into::into)
                    .collect();
                Ok(PaginatedResponse {
                    count: paginated_response.count,
                    next: paginated_response.next,
                    previous: paginated_response.previous,
                    results: director_dtos,
                })
            }
            Err(err) => {
                tracing::error!("Error retrieving paginated director list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all directors.
    async fn get_directors(&self) -> Result<Vec<DirectorDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(directors) => {
                let director_dtos: Vec<DirectorDto> =
                    directors.into_iter().map(Into::into).collect();
                Ok(director_dtos)
            }
            Err(err) => {
                tracing::error!("Error fetching directors: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new director.
    async fn create_director(
        &self,
        create_dto: CreateDirectorDto,
    ) -> Result<DirectorDto, AppError> {
        let txn = self.db.begin().await?;

        let director_id = match self.repo.create(&txn, create_dto).await {
            Ok(director_id) => director_id,
            Err(err) => {
                tracing::error!("Error creating director: {err}");
                txn.rollback().await?;
                return Err(AppError::DatabaseError(err));
            }
        };

        txn.commit().await?;

        match self.repo.find_by_id(&self.db, director_id).await {
            Ok(Some(director)) => Ok(DirectorDto::from(director)),
            Ok(None) => Err(AppError::NotFound("Director not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving director: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing director.
    async fn update_director(
        &self,
        id: i64,
        payload: UpdateDirectorDto,
    ) -> Result<DirectorDto, AppError> {
        let txn = self.db.begin().await?;

        match self.repo.update(&txn, id, payload).await {
            Ok(Some(director)) => {
                txn.commit().await?;
                Ok(DirectorDto::from(director))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Director not found".into()))
            }
            Err(err) => {
                tracing::error!("Error updating director: {err}");
                txn.rollback().await?;
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a director by their ID.
    async fn delete_director(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await?;
                Ok("Director deleted".into())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Director not found".into()))
            }
            Err(err) => {
                tracing::error!("Error deleting director: {err}");
                txn.rollback().await?;
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by directors.
    async fn get_director_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_director_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
