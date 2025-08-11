//! `IdolParticipation` entity
//!
//! Junction table for Idol and Record many-to-many relationship

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as IdolParticipationEntity;
pub use Model as IdolParticipationModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "idol_participation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub idol_id: i64,
    pub record_id: String,
    pub manual: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::idol::Entity",
        from = "Column::IdolId",
        to = "super::idol::Column::Id"
    )]
    Idol,
    #[sea_orm(
        belongs_to = "super::record::Entity",
        from = "Column::RecordId",
        to = "super::record::Column::Id"
    )]
    Record,
}

impl Related<super::idol::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Idol.def()
    }
}

impl Related<super::record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Record.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
