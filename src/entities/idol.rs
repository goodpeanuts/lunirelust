//! Idol entity
//!
//! Represents idols in the system

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as IdolEntity;
pub use Model as IdolModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "idol")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub link: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::idol_participation::Entity")]
    IdolParticipation,
}

impl Related<super::record::Entity> for Entity {
    fn to() -> RelationDef {
        super::idol_participation::Relation::Record.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::idol_participation::Relation::Idol.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
