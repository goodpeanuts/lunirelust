use crate::{
    common::error::AppError,
    domains::{
        luna::dto::{PaginatedResponse, PaginationQuery},
        user::{
            domain::{
                model::user_interaction::InteractionStatus,
                repository::interaction_repo::InteractionRepository,
                service::interaction_service::InteractionServiceTrait,
            },
            infra::impl_repository::interaction_repo::InteractionRepo,
        },
    },
};
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::{collections::HashMap, sync::Arc};

/// Service struct for handling user-record interaction operations.
#[derive(Clone)]
pub struct InteractionService {
    db: DatabaseConnection,
    repo: Arc<dyn InteractionRepository + Send + Sync>,
}

#[async_trait]
impl InteractionServiceTrait for InteractionService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn InteractionServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(InteractionRepo),
        })
    }

    async fn toggle_like(&self, user_id: &str, record_id: &str) -> Result<bool, AppError> {
        self.repo
            .toggle_like(&self.db, user_id, record_id)
            .await
            .map_err(AppError::DatabaseError)
    }

    async fn mark_viewed(&self, user_id: &str, record_id: &str) -> Result<(), AppError> {
        self.repo
            .mark_viewed(&self.db, user_id, record_id)
            .await
            .map_err(AppError::DatabaseError)
    }

    async fn batch_get_status(
        &self,
        user_id: &str,
        record_ids: &[String],
    ) -> Result<HashMap<String, InteractionStatus>, AppError> {
        self.repo
            .batch_get_status(&self.db, user_id, record_ids)
            .await
            .map_err(AppError::DatabaseError)
    }

    async fn get_viewed_record_ids_paginated(
        &self,
        user_id: &str,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<String>, AppError> {
        let page_size = pagination
            .limit
            .filter(|&l| l > 0)
            .unwrap_or(crate::common::config::DEFAULT_PAGE_SIZE as i64)
            as u64;
        let current_offset = pagination.offset.unwrap_or(0).max(0) as u64;

        let (ids, total) = self
            .repo
            .find_viewed_record_ids_paginated(&self.db, user_id, page_size, current_offset)
            .await
            .map_err(AppError::DatabaseError)?;

        let next_offset = current_offset + page_size;
        let next = if next_offset < total {
            Some(format!("?limit={page_size}&offset={next_offset}"))
        } else {
            None
        };
        let previous = if current_offset > 0 {
            Some(format!(
                "?limit={page_size}&offset={}",
                current_offset.saturating_sub(page_size)
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total as i64,
            next,
            previous,
            results: ids,
        })
    }
}
