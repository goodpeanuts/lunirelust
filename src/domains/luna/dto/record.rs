use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::Record;

use super::{
    director::DirectorDto,
    genre::RecordGenreDto,
    idol::{CreateIdolParticipationDto, IdolParticipationDto},
    label::LabelDto,
    link::{CreateLinkDto, LinkDto},
    series::SeriesDto,
    studio::StudioDto,
    CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateSeriesDto,
    CreateStudioDto, UpdateGenreDto,
};

// Record DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecordDto {
    pub id: String,
    pub title: String,
    pub date: Date,
    pub duration: i32,
    pub director: DirectorDto,
    pub studio: StudioDto,
    pub label: LabelDto,
    pub series: SeriesDto,
    pub genres: Vec<RecordGenreDto>,
    pub idols: Vec<IdolParticipationDto>,
    pub has_links: bool,
    pub links: Vec<LinkDto>,
    pub permission: i32,
    pub local_img_count: i32,
    pub create_time: Date,
    pub update_time: Date,
    pub creator: String,
    pub modified_by: String,
}

impl From<Record> for RecordDto {
    fn from(record: Record) -> Self {
        Self {
            id: record.id,
            title: record.title,
            date: record.date,
            duration: record.duration,
            director: DirectorDto::from(record.director),
            studio: StudioDto::from(record.studio),
            label: LabelDto::from(record.label),
            series: SeriesDto::from(record.series),
            genres: record
                .genres
                .into_iter()
                .map(RecordGenreDto::from)
                .collect(),
            idols: record
                .idols
                .into_iter()
                .map(IdolParticipationDto::from)
                .collect(),
            has_links: record.has_links,
            links: record.links.into_iter().map(LinkDto::from).collect(),
            permission: record.permission,
            local_img_count: record.local_img_count,
            create_time: record.create_time,
            update_time: record.update_time,
            creator: record.creator,
            modified_by: record.modified_by,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchRecordDto {
    pub id: Option<String>,
    pub title: Option<String>,
    pub director_id: Option<i64>,
    pub studio_id: Option<i64>,
    pub label_id: Option<i64>,
    pub series_id: Option<i64>,
    pub search: Option<String>, // For search term parameter
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateRecordDto {
    #[validate(length(
        min = 1,
        max = 255,
        message = "ID must be between 1 and 255 characters"
    ))]
    pub id: String,
    #[validate(length(max = 1024, message = "Title cannot exceed 1024 characters"))]
    pub title: String,
    pub date: Date,
    pub duration: i32,
    pub director: Option<CreateDirectorDto>,
    pub studio: Option<CreateStudioDto>,
    pub label: Option<CreateLabelDto>,
    pub series: Option<CreateSeriesDto>,
    pub genres: Vec<CreateGenreDto>,
    pub idols: Vec<CreateIdolDto>,
    pub has_links: bool,
    pub links: Vec<CreateLinkDto>,
    pub permission: i32,
    pub local_img_count: i32,
    pub creator: String,
    pub modified_by: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateRecordDto {
    #[validate(length(max = 1024, message = "Title cannot exceed 1024 characters"))]
    pub title: String,
    pub date: Date,
    pub duration: i32,
    pub director_id: i64,
    pub studio_id: i64,
    pub label_id: i64,
    pub series_id: i64,
    pub genres: Vec<UpdateGenreDto>,
    pub idols: Vec<CreateIdolParticipationDto>,
    pub has_links: bool,
    pub links: Vec<CreateLinkDto>,
    pub permission: i32,
    pub local_img_count: i32,
    pub modified_by: String,
}
