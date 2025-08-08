use crate::domains::user::{
    domain::{model::User, repository::UserRepository},
    dto::user_dto::{CreateUserMultipartDto, SearchUserDto, UpdateUserDto},
};
use crate::entities::{uploaded_files, users, UsersEntity};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ActiveValue::NotSet, ColumnTrait as _, DatabaseConnection,
    DatabaseTransaction, DbErr, EntityTrait as _, QueryFilter as _, Set,
};
use uuid::Uuid;

pub struct UserRepo;

#[async_trait]
impl UserRepository for UserRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<User>, DbErr> {
        let users_models = UsersEntity::find().all(db).await?;

        let mut users = Vec::new();
        for user_model in users_models {
            let file_model = uploaded_files::Entity::find()
                .filter(uploaded_files::Column::UserId.eq(user_model.id.clone()))
                .filter(uploaded_files::Column::FileType.eq("profile_picture"))
                .one(db)
                .await?;

            let domain_user = User::from((user_model, file_model));

            users.push(domain_user);
        }

        Ok(users)
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_user_dto: SearchUserDto,
    ) -> Result<Vec<User>, DbErr> {
        let mut query = UsersEntity::find();

        if let Some(id) = search_user_dto
            .id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            query = query.filter(users::Column::Id.eq(id));
        }

        if let Some(username) = search_user_dto
            .username
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            query = query.filter(users::Column::Username.like(format!("%{username}%")));
        }

        let users_with_files: Vec<(users::Model, Option<uploaded_files::Model>)> = query
            .find_also_related(uploaded_files::Entity)
            .filter(
                sea_orm::Condition::any()
                    .add(uploaded_files::Column::FileType.eq("profile_picture"))
                    .add(uploaded_files::Column::FileType.is_null()),
            )
            .all(db)
            .await?;

        Ok(users_with_files.into_iter().map(User::from).collect())
    }

    async fn find_by_id(&self, db: &DatabaseConnection, id: String) -> Result<Option<User>, DbErr> {
        let user_with_files: Option<(users::Model, Option<uploaded_files::Model>)> =
            UsersEntity::find_by_id(id)
                .find_also_related(uploaded_files::Entity)
                .filter(
                    sea_orm::Condition::any()
                        .add(uploaded_files::Column::FileType.eq("profile_picture"))
                        .add(uploaded_files::Column::FileType.is_null()),
                )
                .one(db)
                .await?;

        Ok(user_with_files.map(User::from))
    }

    async fn create(
        &self,
        txn: &DatabaseTransaction,
        user: CreateUserMultipartDto,
    ) -> Result<String, DbErr> {
        let id = Uuid::new_v4().to_string();

        let user_active_model = users::ActiveModel {
            id: Set(id.clone()),
            username: Set(user.username),
            email: Set(user.email),
            created_by: Set(Some(user.modified_by.clone())),
            created_at: NotSet,
            modified_by: Set(Some(user.modified_by)),
            modified_at: NotSet,
        };

        user_active_model.insert(txn).await?;
        Ok(id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: String,
        user: UpdateUserDto,
    ) -> Result<Option<User>, DbErr> {
        // Check if user exists
        let existing_user = UsersEntity::find()
            .filter(users::Column::Id.eq(id.clone()))
            .one(txn)
            .await?;

        if let Some(existing) = existing_user {
            let mut user_active_model: users::ActiveModel = existing.into();
            user_active_model.username = Set(user.username);
            user_active_model.email = Set(user.email);
            user_active_model.modified_by = Set(Some(user.modified_by));

            let updated_user = user_active_model.update(txn).await?;
            let file_model = uploaded_files::Entity::find()
                .filter(uploaded_files::Column::UserId.eq(id.clone()))
                .filter(
                    sea_orm::Condition::any()
                        .add(uploaded_files::Column::FileType.eq("profile_picture"))
                        .add(uploaded_files::Column::FileType.is_null()),
                )
                .one(txn)
                .await?;
            return Ok(Some(User::from((updated_user, file_model))));
        }

        Ok(None)
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: String) -> Result<bool, DbErr> {
        let result = UsersEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }
}
