use crate::entities::links;
use sea_orm::entity::prelude::*;

/// Domain model representing a link in the application.
#[derive(Debug, Clone)]
pub struct Link {
    pub id: i64,
    pub record_id: String,
    pub name: String,
    pub size: Decimal,
    pub date: Date,
    pub link: String,
    pub star: bool,
}

impl From<links::Model> for Link {
    fn from(link: links::Model) -> Self {
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
