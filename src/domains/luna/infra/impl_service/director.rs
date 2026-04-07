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
    db: DatabaseConnection,
    repo: Arc<dyn DirectorRepository + Send + Sync>,
}

#[async_trait]
impl DirectorServiceTrait for DirectorService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn DirectorServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(DirectorRepo {}),
        })
    }

    async fn get_director_by_id(&self, id: i64) -> Result<DirectorDto, AppError> {
        self.repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(DirectorDto::from)
            .ok_or_else(|| AppError::NotFound("Director not found".into()))
    }

    async fn get_director_list(
        &self,
        search_dto: SearchDirectorDto,
    ) -> Result<Vec<DirectorDto>, AppError> {
        let directors = self.repo.find_list(&self.db, search_dto).await?;
        Ok(directors.into_iter().map(Into::into).collect())
    }

    async fn get_director_list_paginated(
        &self,
        search_dto: SearchDirectorDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<DirectorDto>, AppError> {
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

    async fn get_directors(&self) -> Result<Vec<DirectorDto>, AppError> {
        let directors = self.repo.find_all(&self.db).await?;
        Ok(directors.into_iter().map(Into::into).collect())
    }

    async fn create_director(
        &self,
        create_dto: CreateDirectorDto,
    ) -> Result<DirectorDto, AppError> {
        let txn = self.db.begin().await?;
        let director_id = match self.repo.create(&txn, create_dto).await {
            Ok(id) => id,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };
        txn.commit().await?;
        self.get_director_by_id(director_id).await
    }

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
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

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
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn get_director_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_director_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
