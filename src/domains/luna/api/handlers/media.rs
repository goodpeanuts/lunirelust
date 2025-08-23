use crate::common::dto::RestApiResponse;
use crate::common::{app_state::AppState, error::AppError};
use crate::domains::luna::dto::{ImageData, MediaAccessDto, UploadImageDto};
use axum::extract::Multipart;
use axum::response::IntoResponse;
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

/// Upload images for a specific record ID
///
/// This endpoint accepts multipart form data with image files and uploads them
/// to the private assets directory under the subdirectory named by the ID.
/// Only uploads files that don't already exist (no overwriting).
#[utoipa::path(
    post,
    path = "/cards/media/upload",
    request_body(
        content = String,
        description = "Multipart form data with 'id' field and image files",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "Images uploaded successfully", body = String),
        (status = 400, description = "Bad request - invalid data or record ID not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Media"
)]
pub async fn upload_images(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut id: Option<String> = None;
    let mut images: Vec<ImageData> = Vec::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::ValidationError(format!("Failed to parse multipart form data: {e}"))
    })? {
        let field_name = field.name().unwrap_or("").to_owned();

        if field_name == "id" {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::ValidationError(format!("Failed to read id field: {e}")))?;
            id = Some(String::from_utf8(data.to_vec()).map_err(|e| {
                AppError::ValidationError(format!("Invalid UTF-8 in id field: {e}"))
            })?);
        } else if field_name.starts_with("file") {
            let filename = field.file_name().unwrap_or("unknown").to_owned();
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_owned();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::ValidationError(format!("Failed to read file data: {e}")))?;

            // Extract filename without extension for the name
            let name = filename.split('.').next().unwrap_or(&filename).to_owned();

            images.push(ImageData {
                name,
                mime: content_type,
                bytes: data.to_vec(),
            });
        }
    }

    let id =
        id.ok_or_else(|| AppError::ValidationError("Missing 'id' field in form data".to_owned()))?;

    if images.is_empty() {
        return Err(AppError::ValidationError(
            "No image files provided".to_owned(),
        ));
    }

    // Check if the record ID exists in the database
    match state
        .luna_service
        .record_service()
        .get_record_by_id(&id)
        .await
    {
        Ok(_) => {} // Record exists, continue
        Err(AppError::NotFound(_)) => {
            return Err(AppError::ValidationError(format!(
                "Record with ID '{id}' not found"
            )));
        }
        Err(e) => return Err(e), // Other database errors
    }

    let upload_dto = UploadImageDto { id, files: images };

    let uploaded_count = state
        .luna_service
        .file_service()
        .upload_images(upload_dto)
        .await?;

    Ok(RestApiResponse::success(format!(
        "Successfully uploaded {uploaded_count} image(s)"
    )))
}
