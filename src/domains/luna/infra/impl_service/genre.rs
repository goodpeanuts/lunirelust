use crate::domains::search::SearchEntityType;
use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{GenreRepository, GenreServiceTrait},
        dto::{
            CreateGenreDto, EntityCountDto, GenreDto, PaginatedResponse, PaginationQuery,
            SearchGenreDto, UpdateGenreDto,
        },
        infra::{search_outbox, GenreRepo},
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling genre-related operations.
#[derive(Clone)]
pub struct GenreService {
    db: DatabaseConnection,
    repo: Arc<dyn GenreRepository + Send + Sync>,
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
        let entity_name = create_dto.name.clone();
        let (genre_id, was_created) = match self.repo.create(&txn, create_dto).await {
            Ok(pair) => pair,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        if was_created {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Genre,
                genre_id,
                &entity_name,
                Vec::new(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        self.get_genre_by_id(genre_id).await
    }

    async fn update_genre(
        &self,
        id: i64,
        update_dto: UpdateGenreDto,
    ) -> Result<GenreDto, AppError> {
        let txn = self.db.begin().await?;

        let pre_affected =
            search_outbox::find_affected_record_ids(&txn, SearchEntityType::Genre, id)
                .await
                .map_err(AppError::DatabaseError)?;

        let genre = match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(g)) => g,
            Ok(None) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Genre not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        let surviving_id = genre.id;
        if surviving_id != id {
            search_outbox::outbox_entity_delete(&txn, SearchEntityType::Genre, id, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
            let mut all_affected = pre_affected.clone();
            let surviving_affected = search_outbox::find_affected_record_ids(
                &txn,
                SearchEntityType::Genre,
                surviving_id,
            )
            .await
            .map_err(AppError::DatabaseError)?;
            all_affected.extend(surviving_affected);
            all_affected.sort_unstable();
            all_affected.dedup();
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Genre,
                surviving_id,
                &genre.name,
                all_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &all_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        } else {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Genre,
                id,
                &genre.name,
                pre_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &pre_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        Ok(GenreDto::from(genre))
    }

    async fn delete_genre(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;

        let affected = search_outbox::find_affected_record_ids(&txn, SearchEntityType::Genre, id)
            .await
            .map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {}
            Ok(false) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Genre not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        }

        search_outbox::outbox_entity_delete(&txn, SearchEntityType::Genre, id, affected.clone())
            .await
            .map_err(AppError::DatabaseError)?;
        search_outbox::outbox_fanout_records(&txn, &affected)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await?;
        Ok("Genre deleted successfully".to_owned())
    }

    async fn get_genre_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_genre_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
