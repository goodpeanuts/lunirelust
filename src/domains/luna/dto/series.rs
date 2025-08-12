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
    pub manual: bool,
}

impl From<Series> for SeriesDto {
    fn from(series: Series) -> Self {
        Self {
            id: series.id,
            name: series.name,
            link: series.link,
            manual: series.manual,
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
    pub link: Option<String>,
    pub manual: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateSeriesDto {
    pub id: i64,
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: Option<String>,
    pub link: Option<String>,
    pub manual: Option<bool>,
}
