use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Common pagination and search DTOs

/// Pagination parameters for list endpoints.
///
/// **Page-aligned offset semantics:** Internally, `offset` is converted to a page number
/// via `page_num = offset / limit`. This means offsets snap to page boundaries:
/// e.g. with `limit=10`, `offset=15` returns the same page as `offset=10` (items 10–19).
/// `limit` must be > 0.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationQuery {
    /// Maximum number of items per page. Defaults to [`DEFAULT_PAGE_SIZE`](crate::common::config::DEFAULT_PAGE_SIZE).
    #[serde(default)]
    pub limit: Option<i64>,

    /// Zero-based byte offset into the result set, snapped to the nearest page boundary.
    /// Defaults to 0.
    #[serde(default)]
    pub offset: Option<i64>,

    /// When true, only return records the authenticated user has liked.
    #[serde(default)]
    pub liked_only: Option<bool>,
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

/// User context for filtering records by interaction status.
/// Passed through service/repo layers when a user wants liked-only results.
#[derive(Debug, Clone)]
pub struct UserFilter {
    pub user_id: String,
    pub liked_only: bool,
}
