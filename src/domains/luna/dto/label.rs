use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::Label;

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
