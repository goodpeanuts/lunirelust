use crate::entities::label;

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
