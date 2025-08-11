use crate::entities::studio;

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
