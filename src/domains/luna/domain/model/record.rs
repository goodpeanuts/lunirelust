use sea_orm::entity::prelude::*;

/// Domain model representing a record in the application.
#[derive(Debug, Clone)]
pub struct Record {
    pub id: String,
    pub title: String,
    pub date: Date,
    pub duration: i32,
    pub director: super::director::Director,
    pub studio: super::studio::Studio,
    pub label: super::label::Label,
    pub series: super::series::Series,
    pub genres: Vec<super::genre::RecordGenre>,
    pub idols: Vec<super::idol::IdolParticipation>,
    pub has_links: bool,
    pub links: Vec<super::links::Link>,
    pub permission: i32,
    pub local_img_count: i32,
    pub create_time: Date,
    pub update_time: Date,
    pub creator: String,
    pub modified_by: String,
}
