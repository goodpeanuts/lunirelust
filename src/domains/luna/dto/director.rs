use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::Director;

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

// TODO: Update params should be adjusted
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateDirectorDto {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    #[validate(length(min = 1, message = "Link cannot be empty"))]
    pub link: String,
}
