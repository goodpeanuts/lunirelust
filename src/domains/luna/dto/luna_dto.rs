use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::model::{
    Director, Genre, Idol, IdolParticipation, Label, Link, Record, RecordGenre, Series, Studio,
};

// Common pagination and search DTOs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub count: i64,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

// Count DTOs for statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityCountDto {
    pub id: i64,
    pub name: String,
    pub count: i64,
}

// Director DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DirectorDto {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<Director> for DirectorDto {
    fn from(director: Director) -> Self {
        Self {
            id: director.id,
            name: director.name,
            link: director.link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchDirectorDto {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateDirectorDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateDirectorDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

// Genre DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenreDto {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<Genre> for GenreDto {
    fn from(genre: Genre) -> Self {
        Self {
            id: genre.id,
            name: genre.name,
            link: genre.link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchGenreDto {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateGenreDto {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateGenreDto {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

// Idol DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdolDto {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<Idol> for IdolDto {
    fn from(idol: Idol) -> Self {
        Self {
            id: idol.id,
            name: idol.name,
            link: idol.link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchIdolDto {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub link: Option<String>,
    pub search: Option<String>, // For search term parameter
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateIdolDto {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateIdolDto {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

// Label DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LabelDto {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<Label> for LabelDto {
    fn from(label: Label) -> Self {
        Self {
            id: label.id,
            name: label.name,
            link: label.link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchLabelDto {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateLabelDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateLabelDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

// Studio DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StudioDto {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<Studio> for StudioDto {
    fn from(studio: Studio) -> Self {
        Self {
            id: studio.id,
            name: studio.name,
            link: studio.link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchStudioDto {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateStudioDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateStudioDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

// Series DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SeriesDto {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<Series> for SeriesDto {
    fn from(series: Series) -> Self {
        Self {
            id: series.id,
            name: series.name,
            link: series.link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchSeriesDto {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateSeriesDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateSeriesDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}

// Link DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkDto {
    pub id: i64,
    pub record_id: String,
    pub name: String,
    #[schema(value_type = String)]
    pub size: Decimal,
    pub date: Date,
    pub link: String,
    pub star: bool,
}

impl From<Link> for LinkDto {
    fn from(link: Link) -> Self {
        Self {
            id: link.id,
            record_id: link.record_id,
            name: link.name,
            size: link.size,
            date: link.date,
            link: link.link,
            star: link.star,
        }
    }
}

// Record related DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecordGenreDto {
    pub genre: GenreDto,
    pub manual: bool,
}

impl From<RecordGenre> for RecordGenreDto {
    fn from(record_genre: RecordGenre) -> Self {
        Self {
            genre: GenreDto::from(record_genre.genre),
            manual: record_genre.manual,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdolParticipationDto {
    pub idol: IdolDto,
    pub manual: bool,
}

impl From<IdolParticipation> for IdolParticipationDto {
    fn from(idol_participation: IdolParticipation) -> Self {
        Self {
            idol: IdolDto::from(idol_participation.idol),
            manual: idol_participation.manual,
        }
    }
}

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
    pub director_id: i64,
    pub studio_id: i64,
    pub label_id: i64,
    pub series_id: i64,
    pub genres: Vec<CreateRecordGenreDto>,
    pub idols: Vec<CreateIdolParticipationDto>,
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
    pub genres: Vec<CreateRecordGenreDto>,
    pub idols: Vec<CreateIdolParticipationDto>,
    pub has_links: bool,
    pub links: Vec<CreateLinkDto>,
    pub permission: i32,
    pub local_img_count: i32,
    pub modified_by: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateRecordGenreDto {
    pub genre_id: i64,
    pub manual: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateIdolParticipationDto {
    pub idol_id: i64,
    pub manual: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateLinkDto {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: String,
    #[schema(value_type = String)]
    pub size: Decimal,
    pub date: Date,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
    pub star: bool,
}
