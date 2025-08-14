use crate::common::{app_state::AppState, error::AppError};
use crate::domains::luna::dto::MediaAccessDto;
use axum::{
    extract::{Path, Query, State},
    response::Response,
};
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct MediaPathParams {
    /// The unique identifier for the media
    pub id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct MediaQueryParams {
    /// Optional sequence number for the media file
    pub n: Option<u32>,
}

/// Serves media files (images) for luna cards
///
/// This endpoint serves jpg images based on the provided ID and optional sequence number.
/// - If `n` is provided, it returns `{id}_{n}.jpg`
/// - If `n` is not provided, it returns `{id}.jpg`
///
/// Files are looked up in the configured private assets directory under the subdirectory named by the ID.
#[utoipa::path(
    get,
    path = "/cards/media/{id}",
    params(
        MediaPathParams,
        MediaQueryParams,
    ),
    responses(
        (status = 200, description = "Media file served successfully", content_type = "image/jpg"),
        (status = 404, description = "Media file or directory not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Media"
)]
pub async fn serve_media(
    State(state): State<AppState>,
    Path(path_params): Path<MediaPathParams>,
    Query(query_params): Query<MediaQueryParams>,
) -> Result<Response, AppError> {
    let media_dto = MediaAccessDto::new(path_params.id, query_params.n);

    state
        .luna_service
        .file_service()
        .serve_media_file(media_dto)
        .await
}

/// Alternative endpoint that accepts `n` as a path parameter
/// This endpoint uses two separate path parameters for cleaner URL structure
pub async fn serve_media_with_number(
    State(state): State<AppState>,
    Path((id, n)): Path<(String, u32)>,
) -> Result<Response, AppError> {
    let media_dto = MediaAccessDto::new(id, Some(n));

    state
        .luna_service
        .file_service()
        .serve_media_file(media_dto)
        .await
}
