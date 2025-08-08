use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, JoinType, QueryFilter as _, QuerySelect as _, RelationTrait as _, Set,
};

use crate::domains::auth::domain::model::UserAuth;
use crate::domains::auth::domain::repository::UserAuthRepository;
use crate::entities::{user_auth, users};

pub struct UserAuthRepo;

impl UserAuthRepo {
    fn entity_to_model(entity: user_auth::Model) -> UserAuth {
        UserAuth {
            user_id: entity.user_id,
            password_hash: entity.password_hash,
        }
    }
}

#[async_trait]
impl UserAuthRepository for UserAuthRepo {
    async fn find_by_user_name(
        &self,
        db: &DatabaseConnection,
        user_name: String,
    ) -> Result<Option<UserAuth>, DbErr> {
        let result = user_auth::Entity::find()
            .join(JoinType::InnerJoin, user_auth::Relation::Users.def())
            .filter(users::Column::Username.eq(user_name))
            .one(db)
            .await?
            .map(Self::entity_to_model);

        Ok(result)
    }

    async fn create(&self, tx: &DatabaseTransaction, user_auth: UserAuth) -> Result<(), DbErr> {
        let active_user_auth = user_auth::ActiveModel {
            user_id: Set(user_auth.user_id),
            password_hash: Set(user_auth.password_hash),
            created_at: Set(Some(chrono::Utc::now())),
            modified_at: Set(Some(chrono::Utc::now())),
        };

        active_user_auth.insert(tx).await?;
        Ok(())
    }
}
