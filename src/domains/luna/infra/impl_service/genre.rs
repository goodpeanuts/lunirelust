use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{GenreRepository, GenreServiceTrait},
        dto::{
            CreateGenreDto, EntityCountDto, GenreDto, PaginatedResponse, PaginationQuery,
            SearchGenreDto, UpdateGenreDto,
        },
        infra::GenreRepo,
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling genre-related operations.
#[derive(Clone)]
pub struct GenreService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn GenreRepository + Send + Sync>,
}

#[async_trait]
impl GenreServiceTrait for GenreService {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn GenreServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(GenreRepo {}),
        })
    }

    /// Retrieves a genre by their ID.
    async fn get_genre_by_id(&self, id: i64) -> Result<GenreDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(genre)) => Ok(GenreDto::from(genre)),
            Ok(None) => Err(AppError::NotFound("Genre not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves genre list by condition
    async fn get_genre_list(&self, search_dto: SearchGenreDto) -> Result<Vec<GenreDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(genres) => {
                let genre_dtos: Vec<GenreDto> = genres.into_iter().map(Into::into).collect();
                Ok(genre_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving genre list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves genre list with pagination
    async fn get_genre_list_paginated(
        &self,
        search_dto: SearchGenreDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<GenreDto>, AppError> {
        match self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
        {
            Ok(paginated_response) => {
                let genre_dtos: Vec<GenreDto> = paginated_response
                    .results
                    .into_iter()
                    .map(Into::into)
                    .collect();
                Ok(PaginatedResponse {
                    count: paginated_response.count,
                    next: paginated_response.next,
                    previous: paginated_response.previous,
                    results: genre_dtos,
                })
            }
            Err(err) => {
                tracing::error!("Error retrieving paginated genre list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all genres from the database.
    async fn get_genres(&self) -> Result<Vec<GenreDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(genres) => {
                let genre_dtos: Vec<GenreDto> = genres.into_iter().map(Into::into).collect();
                Ok(genre_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving all genres: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new genre.
    async fn create_genre(&self, create_dto: CreateGenreDto) -> Result<GenreDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.create(&txn, create_dto).await {
            Ok(genre_id) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                self.get_genre_by_id(genre_id).await
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error creating genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing genre.
    async fn update_genre(
        &self,
        id: i64,
        update_dto: UpdateGenreDto,
    ) -> Result<GenreDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(genre)) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok(GenreDto::from(genre))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Genre not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error updating genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a genre by their ID.
    async fn delete_genre(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok("Genre deleted successfully".to_owned())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Genre not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error deleting genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by genres.
    async fn get_genre_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_genre_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
