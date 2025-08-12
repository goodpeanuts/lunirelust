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
    pub manual: bool,
}

impl From<Label> for LabelDto {
    fn from(label: Label) -> Self {
        Self {
            id: label.id,
            name: label.name,
            link: label.link,
            manual: label.manual,
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
    pub link: Option<String>,
    pub manual: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateLabelDto {
    pub id: i64,
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: Option<String>,
    pub link: Option<String>,
    pub manual: Option<bool>,
}
