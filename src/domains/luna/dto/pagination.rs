use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Common pagination and search DTOs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub count: i64,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}
