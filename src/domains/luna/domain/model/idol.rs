use crate::entities::idol;

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

/// Domain model representing an idol participation in a record.
#[derive(Debug, Clone)]
pub struct IdolParticipation {
    pub idol: Idol,
    pub manual: bool,
}
