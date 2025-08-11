use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::{Genre, RecordGenre};

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
