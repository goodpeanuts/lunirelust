use crate::domains::search::SearchEntityType;
use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{LabelRepository, LabelServiceTrait},
        dto::{
            CreateLabelDto, EntityCountDto, LabelDto, PaginatedResponse, PaginationQuery,
            SearchLabelDto, UpdateLabelDto,
        },
        infra::{search_outbox, LabelRepo},
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
        let entity_name = create_dto.name.clone();
        let (label_id, was_created) = match self.repo.create(&txn, create_dto).await {
            Ok(pair) => pair,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        if was_created {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Label,
                label_id,
                &entity_name,
                Vec::new(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        self.get_label_by_id(label_id).await
    }

    async fn update_label(
        &self,
        id: i64,
        update_dto: UpdateLabelDto,
    ) -> Result<LabelDto, AppError> {
        let txn = self.db.begin().await?;

        let pre_affected =
            search_outbox::find_affected_record_ids(&txn, SearchEntityType::Label, id)
                .await
                .map_err(AppError::DatabaseError)?;

        let label = match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(l)) => l,
            Ok(None) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Label not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        let surviving_id = label.id;
        if surviving_id != id {
            search_outbox::outbox_entity_delete(&txn, SearchEntityType::Label, id, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
            let mut all_affected = pre_affected.clone();
            let surviving_affected = search_outbox::find_affected_record_ids(
                &txn,
                SearchEntityType::Label,
                surviving_id,
            )
            .await
            .map_err(AppError::DatabaseError)?;
            all_affected.extend(surviving_affected);
            all_affected.sort_unstable();
            all_affected.dedup();
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Label,
                surviving_id,
                &label.name,
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
                SearchEntityType::Label,
                id,
                &label.name,
                pre_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &pre_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await?;
        Ok(LabelDto::from(label))
    }

    async fn delete_label(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;

        let affected = search_outbox::find_affected_record_ids(&txn, SearchEntityType::Label, id)
            .await
            .map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {}
            Ok(false) => {
                txn.rollback().await?;
                return Err(AppError::NotFound("Label not found".into()));
            }
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        }

        search_outbox::outbox_entity_delete(&txn, SearchEntityType::Label, id, affected.clone())
            .await
            .map_err(AppError::DatabaseError)?;
        search_outbox::outbox_fanout_records(&txn, &affected)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await?;
        Ok("Label deleted successfully".to_owned())
    }

    async fn get_label_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_label_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
