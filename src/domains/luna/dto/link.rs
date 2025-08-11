use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::Link;

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
