use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::Studio;

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
