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

        let all_studios = self.repo.find_list(&self.db, search_dto).await?;
        let total_count = all_studios.len() as i64;

        let studios: Vec<_> = all_studios
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

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

    async fn get_studios(&self) -> Result<Vec<StudioDto>, AppError> {
        let studios = self.repo.find_all(&self.db).await?;
        Ok(studios.into_iter().map(StudioDto::from).collect())
    }

    async fn create_studio(&self, create_dto: CreateStudioDto) -> Result<StudioDto, AppError> {
        let txn = self.db.begin().await?;
        let studio_id = match self.repo.create(&txn, create_dto).await {
            Ok(id) => id,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };
        txn.commit().await?;
        self.get_studio_by_id(studio_id).await
    }

    async fn update_studio(
        &self,
        id: i64,
        update_dto: UpdateStudioDto,
    ) -> Result<StudioDto, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(studio)) => {
                txn.commit().await?;
                Ok(StudioDto::from(studio))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Studio not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn delete_studio(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await?;
                Ok("Studio deleted successfully".into())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Studio not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn get_studio_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_studio_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}
