//! Genre entity
//!
//! Represents genres in the system

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as GenreEntity;
pub use Model as GenreModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "genre")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub link: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::record_genre::Entity")]
    RecordGenre,
}

impl Related<super::record::Entity> for Entity {
    fn to() -> RelationDef {
        super::record_genre::Relation::Record.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::record_genre::Relation::Genre.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
