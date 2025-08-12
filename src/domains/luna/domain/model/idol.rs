use crate::entities::idol;

/// Domain model representing an idol in the application.
#[derive(Debug, Clone)]
pub struct Idol {
    pub id: i64,
    pub name: String,
    pub link: String,
    pub manual: bool,
}

impl From<idol::Model> for Idol {
    fn from(idol: idol::Model) -> Self {
        Self {
            id: idol.id,
            name: idol.name,
            link: idol.link,
            manual: idol.manual,
        }
    }
}

/// Domain model representing an idol participation in a record.
#[derive(Debug, Clone)]
pub struct IdolParticipation {
    pub idol: Idol,
    pub manual: bool,
}
