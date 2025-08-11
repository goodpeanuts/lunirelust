use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::Series;

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
