//! Search API routes.

use axum::routing::get;
use axum::Router;
use utoipa::OpenApi;

use crate::common::app_state::AppState;
use crate::common::openapi::SecurityAddon;
use crate::domains::search::api::handlers::search_handler::{__path_search, search};
use crate::domains::search::dto::SearchResponse;

#[derive(OpenApi)]
#[openapi(
    paths(
        search,
    ),
    components(schemas(
        SearchResponse,
    )),
    tags(
        (name = "Search", description = "Unified search endpoints")
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon)
)]
pub struct SearchApiDoc;

pub fn search_routes() -> Router<AppState> {
    Router::new().route("/search", get(search))
}
