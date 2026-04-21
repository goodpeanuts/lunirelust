use crate::{
    common::error::AppError,
    domains::user::{
        domain::{
            model::user_interaction::InteractionStatus,
            repository::interaction_repo::InteractionRepository,
            service::interaction_service::InteractionServiceTrait,
        },
        infra::impl_repository::interaction_repo::InteractionRepo,
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
}
