use crate::entities::{director, genre, idol, label, links, series, studio};
use sea_orm::entity::prelude::*;

/// Domain model representing a director in the application.
#[derive(Debug, Clone)]
pub struct Director {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<director::Model> for Director {
    fn from(director: director::Model) -> Self {
        Self {
            id: director.id,
            name: director.name,
            link: director.link,
        }
    }
}

/// Domain model representing a genre in the application.
#[derive(Debug, Clone)]
pub struct Genre {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<genre::Model> for Genre {
    fn from(genre: genre::Model) -> Self {
        Self {
            id: genre.id,
            name: genre.name,
            link: genre.link,
        }
    }
}

/// Domain model representing an idol in the application.
#[derive(Debug, Clone)]
pub struct Idol {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<idol::Model> for Idol {
    fn from(idol: idol::Model) -> Self {
        Self {
            id: idol.id,
            name: idol.name,
            link: idol.link,
        }
    }
}

/// Domain model representing a label in the application.
#[derive(Debug, Clone)]
pub struct Label {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<label::Model> for Label {
    fn from(label: label::Model) -> Self {
        Self {
            id: label.id,
            name: label.name,
            link: label.link,
        }
    }
}

/// Domain model representing a studio in the application.
#[derive(Debug, Clone)]
pub struct Studio {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<studio::Model> for Studio {
    fn from(studio: studio::Model) -> Self {
        Self {
            id: studio.id,
            name: studio.name,
            link: studio.link,
        }
    }
}

/// Domain model representing a series in the application.
#[derive(Debug, Clone)]
pub struct Series {
    pub id: i64,
    pub name: String,
    pub link: String,
}

impl From<series::Model> for Series {
    fn from(series: series::Model) -> Self {
        Self {
            id: series.id,
            name: series.name,
            link: series.link,
        }
    }
}

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

/// Domain model representing a record genre association.
#[derive(Debug, Clone)]
pub struct RecordGenre {
    pub genre: Genre,
    pub manual: bool,
}

/// Domain model representing an idol participation in a record.
#[derive(Debug, Clone)]
pub struct IdolParticipation {
    pub idol: Idol,
    pub manual: bool,
}

/// Domain model representing a record in the application.
#[derive(Debug, Clone)]
pub struct Record {
    pub id: String,
    pub title: String,
    pub date: Date,
    pub duration: i32,
    pub director: Director,
    pub studio: Studio,
    pub label: Label,
    pub series: Series,
    pub genres: Vec<RecordGenre>,
    pub idols: Vec<IdolParticipation>,
    pub has_links: bool,
    pub links: Vec<Link>,
    pub permission: i32,
    pub local_img_count: i32,
    pub create_time: Date,
    pub update_time: Date,
    pub creator: String,
    pub modified_by: String,
}
