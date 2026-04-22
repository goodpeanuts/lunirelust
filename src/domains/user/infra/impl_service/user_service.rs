use crate::{
    common::error::AppError,
    domains::{
        file::{dto::file_dto::UploadFileDto, FileServiceTrait},
        user::{
            domain::{
                repository::user_repo::UserRepository,
                service::{
                    interaction_service::InteractionServiceTrait, user_service::UserServiceTrait,
                },
            },
            dto::user_dto::{CreateUserMultipartDto, SearchUserDto, UpdateUserDto, UserDto},
            infra::{
                impl_repository::user_repo::UserRepo,
                impl_service::interaction_service::InteractionService,
            },
        },
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling user-related operations
#[derive(Clone)]
pub struct UserService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn UserRepository + Send + Sync>,
    pub file_service: Arc<dyn FileServiceTrait>,
    pub interaction_service: Arc<dyn InteractionServiceTrait>,
}

#[async_trait]
impl UserServiceTrait for UserService {
    fn create_service(
        db: DatabaseConnection,
        file_service: Arc<dyn FileServiceTrait>,
    ) -> Arc<dyn UserServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(UserRepo),
            file_service,
            interaction_service: InteractionService::create_service(db),
        })
    }

    async fn get_user_by_id(&self, id: String) -> Result<UserDto, AppError> {
        self.repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(UserDto::from)
            .ok_or_else(|| AppError::NotFound("User not found".into()))
    }

    async fn get_user_list(
        &self,
        search_user_dto: SearchUserDto,
    ) -> Result<Vec<UserDto>, AppError> {
        let users = self.repo.find_list(&self.db, search_user_dto).await?;
        Ok(users.into_iter().map(Into::into).collect())
    }

    async fn get_users(&self) -> Result<Vec<UserDto>, AppError> {
        let users = self.repo.find_all(&self.db).await?;
        Ok(users.into_iter().map(Into::into).collect())
    }

    async fn create_user(
        &self,
        create_user: CreateUserMultipartDto,
        upload_file_dto: Option<&mut UploadFileDto>,
    ) -> Result<UserDto, AppError> {
        let txn = self.db.begin().await?;
        let user_id = match self.repo.create(&txn, create_user).await {
            Ok(id) => id,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };

        if let Some(upload_file_dto) = upload_file_dto {
            upload_file_dto.user_id = Some(user_id.clone());
            if let Err(e) = self
                .file_service
                .process_profile_picture_upload(&txn, upload_file_dto)
                .await
            {
                txn.rollback().await.ok();
                return Err(e);
            }
        }

        txn.commit().await?;

        self.repo
            .find_by_id(&self.db, user_id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(UserDto::from)
            .ok_or_else(|| AppError::NotFound("User not found".into()))
    }

    async fn update_user(&self, id: String, payload: UpdateUserDto) -> Result<UserDto, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.update(&txn, id.clone(), payload).await {
            Ok(Some(user)) => {
                txn.commit().await?;
                Ok(UserDto::from(user))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("User not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn delete_user(&self, id: String) -> Result<String, AppError> {
        let txn = self.db.begin().await?;
        match self.repo.delete(&txn, id.clone()).await {
            Ok(true) => {
                txn.commit().await?;
                Ok("User deleted".into())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("User not found".into()))
            }
            Err(e) => {
                txn.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    fn interaction_service(&self) -> &dyn InteractionServiceTrait {
        &*self.interaction_service
    }
}
