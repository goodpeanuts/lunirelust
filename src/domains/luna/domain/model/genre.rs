use crate::entities::genre;

/// Domain model representing a genre in the application.
#[derive(Debug, Clone)]
pub struct Genre {
    pub id: i64,
    pub name: String,
    pub link: String,
    pub manual: bool,
}

impl From<genre::Model> for Genre {
    fn from(genre: genre::Model) -> Self {
        Self {
            id: genre.id,
            name: genre.name,
            link: genre.link,
            manual: genre.manual,
        }
    }
}

/// Domain model representing a record genre association.
#[derive(Debug, Clone)]
pub struct RecordGenre {
    pub genre: super::genre::Genre,
    pub manual: bool,
}
