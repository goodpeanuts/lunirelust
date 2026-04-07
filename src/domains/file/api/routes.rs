use super::handlers::{
    __path_delete_file, __path_serve_protected_file, delete_file, serve_protected_file,
};
use crate::{common::app_state::AppState, domains::file::dto::file_dto::UploadedFileDto};
use axum::{
    routing::{delete, get},
    Router,
};

use utoipa::OpenApi;

use crate::common::openapi::SecurityAddon;

#[derive(OpenApi)]
#[openapi(
    paths(
        serve_protected_file,
        delete_file,
    ),
    components(schemas(UploadedFileDto)),
    tags(
        (name = "Files", description = "File management endpoints")
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon)
)]
/// `FileApiDoc` is used to generate `OpenAPI` documentation for the file API.
pub struct FileApiDoc;

pub fn file_routes() -> Router<AppState> {
    Router::new()
        .route("/{file_id}", get(serve_protected_file))
        .route("/{file_id}", delete(delete_file))
}
