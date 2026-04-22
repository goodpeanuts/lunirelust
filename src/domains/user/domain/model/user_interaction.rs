use chrono::{DateTime, Utc};

/// Domain model representing a user's interaction with a record.
#[expect(dead_code)]
#[derive(Debug, Clone)]
pub struct UserInteraction {
    pub id: i64,
    pub user_id: String,
    pub record_id: String,
    pub liked: bool,
    pub viewed: bool,
    pub liked_at: Option<DateTime<Utc>>,
    pub viewed_at: Option<DateTime<Utc>>,
}

/// Status of a user's interaction with a specific record.
#[derive(Debug, Clone, Default)]
pub struct InteractionStatus {
    pub liked: bool,
    pub viewed: bool,
}

impl From<crate::entities::user_record_interaction::Model> for UserInteraction {
    fn from(model: crate::entities::user_record_interaction::Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            record_id: model.record_id,
            liked: model.liked,
            viewed: model.viewed,
            liked_at: model.liked_at,
            viewed_at: model.viewed_at,
        }
    }
}
