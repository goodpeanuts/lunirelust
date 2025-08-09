//! `RecordGenre` entity
//!
//! Junction table for Record and Genre many-to-many relationship

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "record_genre")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub record_id: String,
    pub genre_id: i64,
    pub manual: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::record::Entity",
        from = "Column::RecordId",
        to = "super::record::Column::Id"
    )]
    Record,
    #[sea_orm(
        belongs_to = "super::genre::Entity",
        from = "Column::GenreId",
        to = "super::genre::Column::Id"
    )]
    Genre,
}

impl Related<super::record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Record.def()
    }
}

impl Related<super::genre::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Genre.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
