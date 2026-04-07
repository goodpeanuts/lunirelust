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
    fn create_service(db: DatabaseConnection) -> Arc<dyn GenreServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(GenreRepo {}),
        })
    }

    async fn get_genre_by_id(&self, id: i64) -> Result<GenreDto, AppError> {
        self.repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(GenreDto::from)
            .ok_or_else(|| AppError::NotFound("Genre not found".into()))
    }

    async fn get_genre_list(&self, search_dto: SearchGenreDto) -> Result<Vec<GenreDto>, AppError> {
        let genres = self.repo.find_list(&self.db, search_dto).await?;
        Ok(genres.into_iter().map(Into::into).collect())
    }

    async fn get_genre_list_paginated(
        &self,
        search_dto: SearchGenreDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<GenreDto>, AppError> {
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

    async fn get_genres(&self) -> Result<Vec<GenreDto>, AppError> {
        let genres = self.repo.find_all(&self.db).await?;
        Ok(genres.into_iter().map(Into::into).collect())
    }

    async fn create_genre(&self, create_dto: CreateGenreDto) -> Result<GenreDto, AppError> {
        let txn = self.db.begin().await?;
        let genre_id = match self.repo.create(&txn, create_dto).await {
            Ok(id) => id,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };
        txn.commit().await?;
        self.get_genre_by_id(genre_id).await
    }

    async fn update_genre(
        &self,
        id: i64,
        update_dto: UpdateGenreDto,
    ) -> Result<GenreDto, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(genre)) => {
                txn.commit().await?;
                Ok(GenreDto::from(genre))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Genre not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn delete_genre(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await?;
                Ok("Genre deleted successfully".to_owned())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Genre not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn get_genre_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_genre_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
