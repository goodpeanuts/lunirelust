use crate::entities::{uploaded_files, users};
use chrono::{DateTime, Utc};

/// Domain model representing a user in the application.
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_by: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
    pub file_id: Option<String>,
    pub origin_file_name: Option<String>,
}

impl From<(users::Model, Option<uploaded_files::Model>)> for User {
    fn from(user_with_file: (users::Model, Option<uploaded_files::Model>)) -> Self {
        Self {
            id: user_with_file.0.id,
            username: user_with_file.0.username,
            email: Some(user_with_file.0.email),
            created_by: user_with_file.0.created_by,
            created_at: user_with_file.0.created_at,
            modified_by: user_with_file.0.modified_by,
            modified_at: user_with_file.0.modified_at,
            file_id: user_with_file.1.clone().map(|f| f.id),
            origin_file_name: user_with_file.1.map(|f| f.origin_file_name),
        }
    }
}
