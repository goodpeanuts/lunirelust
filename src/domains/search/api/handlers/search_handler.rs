//! Search API handler.

use axum::{
    extract::{Query, State},
    Extension,
};
use serde::Deserialize;

use crate::common::app_state::AppState;
use crate::common::dto::RestApiResponse;
use crate::common::error::AppError;
use crate::common::jwt::Claims;
use crate::domains::search::dto::{SearchQuery, SearchResponse};

/// Search endpoint: GET /cards/search
#[utoipa::path(
    get,
    path = "/cards/search",
    params(
        ("q" = String, Query, description = "Search query string"),
        ("entity_types" = Option<String>, Query, description = "Comma-separated entity types (record,idol,...)"),
        ("director" = Option<String>, Query, description = "Filter by director name"),
        ("studio" = Option<String>, Query, description = "Filter by studio name"),
        ("label" = Option<String>, Query, description = "Filter by label name"),
        ("genre" = Option<String>, Query, description = "Filter by genre name"),
        ("date_from" = Option<String>, Query, description = "Filter from date (inclusive)"),
        ("date_to" = Option<String>, Query, description = "Filter to date (inclusive)"),
        ("limit" = Option<i64>, Query, description = "Max results (default 20)"),
        ("offset" = Option<i64>, Query, description = "Results offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Bad request - empty query"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Search"
)]
pub async fn search(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<SearchParams>,
) -> Result<RestApiResponse<SearchResponse>, AppError> {
    let query = SearchQuery {
        q: params.q,
        entity_types: params.entity_types,
        director: params.director,
        studio: params.studio,
        label: params.label,
        genre: params.genre,
        date_from: params.date_from,
        date_to: params.date_to,
        limit: params.limit,
        offset: params.offset,
    };

    let user_permission = state.search_service.get_user_permission(&claims.sub).await;

    let response = state.search_service.search(query, user_permission).await?;

    Ok(RestApiResponse::success(response))
}

/// Raw search params from query string.
/// Maps directly from the HTTP query string and is then converted into `SearchQuery`.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Search query string (required).
    pub q: String,
    /// Comma-separated entity types to filter by (e.g., "record,idol").
    pub entity_types: Option<String>,
    /// Filter by director name.
    pub director: Option<String>,
    /// Filter by studio name.
    pub studio: Option<String>,
    /// Filter by label name.
    pub label: Option<String>,
    /// Filter by genre name.
    pub genre: Option<String>,
    /// Filter by date range start (inclusive).
    pub date_from: Option<String>,
    /// Filter by date range end (inclusive).
    pub date_to: Option<String>,
    /// Maximum number of results to return (default 20).
    pub limit: Option<i64>,
    /// Number of results to skip (default 0).
    pub offset: Option<i64>,
}
