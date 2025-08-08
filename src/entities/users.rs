//! Users entity for `SeaORM`

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub username: String,
    pub email: String,
    pub created_by: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub modified_by: Option<String>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::devices::Entity")]
    Devices,
    #[sea_orm(has_many = "super::uploaded_files::Entity")]
    UploadedFiles,
    #[sea_orm(has_one = "super::user_auth::Entity")]
    UserAuth,
}

impl Related<super::devices::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Devices.def()
    }
}

impl Related<super::uploaded_files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UploadedFiles.def()
    }
}

impl Related<super::user_auth::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAuth.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
