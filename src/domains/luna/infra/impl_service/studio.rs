use crate::domains::search::SearchEntityType;
use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{StudioRepository, StudioServiceTrait},
        dto::{
            CreateStudioDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchStudioDto,
            StudioDto, UpdateStudioDto,
        },
        infra::{search_outbox, StudioRepo},
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling studio-related operations.
#[derive(Clone)]
pub struct StudioService {
    db: DatabaseConnection,
    repo: Arc<dyn StudioRepository + Send + Sync>,
}

#[async_trait]
impl StudioServiceTrait for StudioService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn StudioServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(StudioRepo),
        })
    }

    async fn get_studio_by_id(&self, id: i64) -> Result<StudioDto, AppError> {
        self.repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(StudioDto::from)
            .ok_or_else(|| AppError::NotFound("Studio not found".into()))
    }

    async fn get_studio_list(
        &self,
        search_dto: SearchStudioDto,
    ) -> Result<Vec<StudioDto>, AppError> {
        let studios = self.repo.find_list(&self.db, search_dto).await?;
        Ok(studios.into_iter().map(StudioDto::from).collect())
    }

    async fn get_studio_list_paginated(
        &self,
        search_dto: SearchStudioDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<StudioDto>, AppError> {
        let paginated = self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await?;
        Ok(PaginatedResponse {
            count: paginated.count,
            next: paginated.next,
            previous: paginated.previous,
            results: paginated.results.into_iter().map(StudioDto::from).collect(),
        })
    }

    async fn get_studios(&self) -> Result<Vec<StudioDto>, AppError> {
        let studios = self.repo.find_all(&self.db).await?;
        Ok(studios.into_iter().map(StudioDto::from).collect())
    }

    async fn create_studio(&self, create_dto: CreateStudioDto) -> Result<StudioDto, AppError> {
        let txn = self.db.begin().await?;
        let entity_name = create_dto.name.clone();
        let (studio_id, was_created) = match self.repo.create(&txn, create_dto).await {
            Ok(pair) => pair,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        if was_created {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Studio,
                studio_id,
                &entity_name,
                Vec::new(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        self.get_studio_by_id(studio_id).await
    }

    async fn update_studio(
        &self,
        id: i64,
        update_dto: UpdateStudioDto,
    ) -> Result<StudioDto, AppError> {
        let txn = self.db.begin().await?;

        let pre_affected =
            search_outbox::find_affected_record_ids(&txn, SearchEntityType::Studio, id)
                .await
                .map_err(AppError::DatabaseError)?;

        let studio = match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Studio not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        let surviving_id = studio.id;
        if surviving_id != id {
            search_outbox::outbox_entity_delete(&txn, SearchEntityType::Studio, id, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
            let mut all_affected = pre_affected.clone();
            let surviving_affected = search_outbox::find_affected_record_ids(
                &txn,
                SearchEntityType::Studio,
                surviving_id,
            )
            .await
            .map_err(AppError::DatabaseError)?;
            all_affected.extend(surviving_affected);
            all_affected.sort_unstable();
            all_affected.dedup();
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Studio,
                surviving_id,
                &studio.name,
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
                SearchEntityType::Studio,
                id,
                &studio.name,
                pre_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &pre_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        Ok(StudioDto::from(studio))
    }

    async fn delete_studio(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;

        let affected = search_outbox::find_affected_record_ids(&txn, SearchEntityType::Studio, id)
            .await
            .map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {}
            Ok(false) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Studio not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        }

        search_outbox::outbox_entity_delete(&txn, SearchEntityType::Studio, id, affected.clone())
            .await
            .map_err(AppError::DatabaseError)?;
        search_outbox::outbox_fanout_records(&txn, &affected)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await?;
        Ok("Studio deleted successfully".into())
    }

    async fn get_studio_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_studio_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
