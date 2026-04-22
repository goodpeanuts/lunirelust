//! User record interaction entity for `SeaORM`
//!
//! Tracks per-user liked and viewed status for records.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as UserRecordInteractionEntity;
pub use Model as UserRecordInteractionModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_record_interaction")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: String,
    pub record_id: String,
    pub liked: bool,
    pub viewed: bool,
    pub liked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub viewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::record::Entity",
        from = "Column::RecordId",
        to = "super::record::Column::Id"
    )]
    Record,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Record.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
