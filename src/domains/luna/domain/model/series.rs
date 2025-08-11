use crate::entities::series;

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
