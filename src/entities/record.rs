//! Record entity
//!
//! Represents records (main content) in the system

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub use Entity as RecordEntity;
pub use Model as RecordModel;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "record")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub title: String,
    pub date: Date,
    pub duration: i32,
    pub director_id: i64,
    pub studio_id: i64,
    pub label_id: i64,
    pub series_id: i64,
    pub has_links: bool,
    pub permission: i32,
    pub local_img_count: i32,
    pub create_time: Date,
    pub update_time: Date,
    pub creator: String,
    pub modified_by: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::director::Entity",
        from = "Column::DirectorId",
        to = "super::director::Column::Id"
    )]
    Director,
    #[sea_orm(
        belongs_to = "super::studio::Entity",
        from = "Column::StudioId",
        to = "super::studio::Column::Id"
    )]
    Studio,
    #[sea_orm(
        belongs_to = "super::label::Entity",
        from = "Column::LabelId",
        to = "super::label::Column::Id"
    )]
    Label,
    #[sea_orm(
        belongs_to = "super::series::Entity",
        from = "Column::SeriesId",
        to = "super::series::Column::Id"
    )]
    Series,
    #[sea_orm(has_many = "super::record_genre::Entity")]
    RecordGenre,
    #[sea_orm(has_many = "super::idol_participation::Entity")]
    IdolParticipation,
    #[sea_orm(has_many = "super::links::Entity")]
    Links,
}

impl Related<super::director::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Director.def()
    }
}

impl Related<super::studio::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Studio.def()
    }
}

impl Related<super::label::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Label.def()
    }
}

impl Related<super::series::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Series.def()
    }
}

impl Related<super::genre::Entity> for Entity {
    fn to() -> RelationDef {
        super::record_genre::Relation::Genre.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::record_genre::Relation::Record.def().rev())
    }
}

impl Related<super::idol::Entity> for Entity {
    fn to() -> RelationDef {
        super::idol_participation::Relation::Idol.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::idol_participation::Relation::Record.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
