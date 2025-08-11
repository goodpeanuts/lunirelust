use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Count DTOs for statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityCountDto {
    pub id: i64,
    pub name: String,
    pub count: i64,
}
