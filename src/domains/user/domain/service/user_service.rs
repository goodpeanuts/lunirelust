use crate::{
    common::error::AppError,
    domains::file::dto::file_dto::UploadFileDto,
    domains::user::dto::user_dto::{CreateUserMultipartDto, SearchUserDto, UpdateUserDto, UserDto},
};

use super::interaction_service::InteractionServiceTrait;
use crate::domains::file::FileServiceTrait;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[async_trait]
/// Trait defining business operations for user management.
pub trait UserServiceTrait: Send + Sync {
    /// constructor for the service.
    fn create_service(
        db: DatabaseConnection,
        file_service: Arc<dyn FileServiceTrait>,
    ) -> Arc<dyn UserServiceTrait>
    where
        Self: Sized;

    /// Retrieves a user by their unique identifier.
    async fn get_user_by_id(&self, id: String) -> Result<UserDto, AppError>;

    /// Retrieves user list by condition
    async fn get_user_list(&self, search_user_dto: SearchUserDto)
        -> Result<Vec<UserDto>, AppError>;

    /// Retrieves all users.
    async fn get_users(&self) -> Result<Vec<UserDto>, AppError>;

    /// Creates a new user with optional profile picture upload.
    async fn create_user(
        &self,
        create_user: CreateUserMultipartDto,
        upload_file_dto: Option<&mut UploadFileDto>,
    ) -> Result<UserDto, AppError>;

    /// Updates an existing user with the given payload.
    async fn update_user(&self, id: String, payload: UpdateUserDto) -> Result<UserDto, AppError>;

    /// Deletes a user by their unique identifier.
    async fn delete_user(&self, id: String) -> Result<String, AppError>;

    /// Get the interaction service.
    fn interaction_service(&self) -> &dyn InteractionServiceTrait;
}
