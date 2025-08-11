//! Links entity
//!
//! Represents download links for records

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as LinksEntity;
pub use Model as LinksModel;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "links")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub record_id: String,
    pub name: String,
    pub size: Decimal,
    pub date: Date,
    pub link: String,
    pub star: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::record::Entity",
        from = "Column::RecordId",
        to = "super::record::Column::Id"
    )]
    Record,
}

impl Related<super::record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Record.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
