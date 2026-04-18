use crate::domains::search::SearchEntityType;
use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{DirectorRepository, DirectorServiceTrait},
        dto::{
            CreateDirectorDto, DirectorDto, EntityCountDto, PaginatedResponse, PaginationQuery,
            SearchDirectorDto, UpdateDirectorDto,
        },
        infra::{search_outbox, DirectorRepo},
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
        let entity_name = create_dto.name.clone();
        let (director_id, was_created) = match self.repo.create(&txn, create_dto).await {
            Ok(pair) => pair,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        if was_created {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Director,
                director_id,
                &entity_name,
                Vec::new(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        self.get_director_by_id(director_id).await
    }

    async fn update_director(
        &self,
        id: i64,
        payload: UpdateDirectorDto,
    ) -> Result<DirectorDto, AppError> {
        let txn = self.db.begin().await?;

        // Capture affected records BEFORE the update, so we still have them
        // if the update triggers a duplicate-merge that deletes this entity.
        let pre_affected =
            search_outbox::find_affected_record_ids(&txn, SearchEntityType::Director, id)
                .await
                .map_err(AppError::DatabaseError)?;

        let director = match self.repo.update(&txn, id, payload).await {
            Ok(Some(d)) => d,
            Ok(None) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Director not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        // Fan-out: find affected records and insert reindex events.
        // Use the surviving entity's ID (may differ from input after merge).
        let surviving_id = director.id;
        if surviving_id != id {
            // Duplicate-merge: the old entity was deleted. Remove its search doc.
            search_outbox::outbox_entity_delete(&txn, SearchEntityType::Director, id, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
            // Records that pointed at the old entity need reindexing (their FK
            // was cascaded). Use the pre-update snapshot since the old row is gone.
            let mut all_affected = pre_affected.clone();
            let surviving_affected = search_outbox::find_affected_record_ids(
                &txn,
                SearchEntityType::Director,
                surviving_id,
            )
            .await
            .map_err(AppError::DatabaseError)?;
            all_affected.extend(surviving_affected);
            all_affected.sort_unstable();
            all_affected.dedup();
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Director,
                surviving_id,
                &director.name,
                all_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &all_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        } else {
            // Normal update: use the pre-update affected records.
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Director,
                id,
                &director.name,
                pre_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &pre_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        Ok(DirectorDto::from(director))
    }

    async fn delete_director(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;

        // Pre-delete snapshot: find affected records BEFORE delete
        let affected =
            search_outbox::find_affected_record_ids(&txn, SearchEntityType::Director, id)
                .await
                .map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {}
            Ok(false) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Director not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        }

        search_outbox::outbox_entity_delete(&txn, SearchEntityType::Director, id, affected.clone())
            .await
            .map_err(AppError::DatabaseError)?;
        search_outbox::outbox_fanout_records(&txn, &affected)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await?;
        Ok("Director deleted".into())
    }

    async fn get_director_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_director_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
