use crate::entities::director;

/// Domain model representing a director in the application.
#[derive(Debug, Clone)]
pub struct Director {
    pub id: i64,
    pub name: String,
    pub link: String,
    pub manual: bool,
}

impl From<director::Model> for Director {
    fn from(director: director::Model) -> Self {
        Self {
            id: director.id,
            name: director.name,
            link: director.link,
            manual: director.manual,
        }
    }
}
