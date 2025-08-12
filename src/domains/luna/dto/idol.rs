use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::{Idol, IdolParticipation};

// Idol DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdolDto {
    pub id: i64,
    pub name: String,
    pub link: String,
    pub manual: bool,
}

impl From<Idol> for IdolDto {
    fn from(idol: Idol) -> Self {
        Self {
            id: idol.id,
            name: idol.name,
            link: idol.link,
            manual: idol.manual,
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
    pub link: Option<String>,
    pub manual: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateIdolDto {
    pub id: i64,
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: Option<String>,
    pub link: Option<String>,
    pub manual: Option<bool>,
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

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateIdolParticipationDto {
    pub idol_id: i64,
    pub manual: bool,
}
