use crate::domains::file::{
    domain::{
        model::{FileType, UploadedFile},
        repository::FileRepository,
    },
    dto::file_dto::CreateFileDto,
};
use crate::entities::uploaded_files;
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, QueryFilter as _, Set,
};
use std::str::FromStr as _;
use uuid::Uuid;

pub struct FileRepo;

impl FileRepo {
    fn entity_to_model(entity: uploaded_files::Model) -> Result<UploadedFile, DbErr> {
        Ok(UploadedFile {
            id: entity.id,
            user_id: entity.user_id,
            file_name: entity.file_name,
            origin_file_name: entity.origin_file_name,
            file_relative_path: entity.file_relative_path,
            file_url: entity.file_url,
            content_type: entity.content_type,
            file_size: entity.file_size,
            file_type: FileType::from_str(&entity.file_type)
                .map_err(|e| DbErr::Type(e.to_string()))?,
            created_by: entity.created_by,
            created_at: entity.created_at.unwrap_or_default(),
            modified_by: entity.modified_by,
            modified_at: entity.modified_at.unwrap_or_default(),
        })
    }
}

#[async_trait]
impl FileRepository for FileRepo {
    async fn create_file(
        &self,
        tx: &DatabaseTransaction,
        file: CreateFileDto,
    ) -> Result<UploadedFile, DbErr> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let user_id = file
            .user_id
            .ok_or_else(|| DbErr::Custom("user_id is required".to_owned()))?;

        let active_file = uploaded_files::ActiveModel {
            id: Set(id.clone()),
            user_id: Set(user_id),
            file_name: Set(file.file_name),
            origin_file_name: Set(file.origin_file_name),
            file_relative_path: Set(file.file_relative_path),
            file_url: Set(file.file_url),
            content_type: Set(file.content_type),
            file_size: Set(file.file_size as i64),
            file_type: Set(file.file_type.to_string()),
            created_by: Set(Some(file.modified_by.clone())),
            created_at: Set(Some(now)),
            modified_by: Set(Some(file.modified_by)),
            modified_at: Set(Some(now)),
        };

        let inserted = active_file.insert(tx).await?;
        Self::entity_to_model(inserted)
    }

    async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Option<UploadedFile>, DbErr> {
        uploaded_files::Entity::find_by_id(id)
            .one(db)
            .await?
            .map(Self::entity_to_model)
            .transpose()
    }

    async fn find_by_user_id(
        &self,
        db: &DatabaseConnection,
        user_id: String,
    ) -> Result<Option<UploadedFile>, DbErr> {
        uploaded_files::Entity::find()
            .filter(uploaded_files::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .map(Self::entity_to_model)
            .transpose()
    }

    async fn delete(&self, tx: &DatabaseTransaction, id: String) -> Result<bool, DbErr> {
        let result = uploaded_files::Entity::delete_by_id(id).exec(tx).await?;

        Ok(result.rows_affected > 0)
    }
}
