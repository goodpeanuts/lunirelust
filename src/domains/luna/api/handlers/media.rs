use crate::common::dto::RestApiResponse;
use crate::common::{app_state::AppState, error::AppError};
use crate::domains::luna::dto::{ImageData, MediaAccessDto, MediaType, UploadImageDto};
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
    let media_dto = MediaAccessDto::new(path_params.id, MediaType::RecordImage, query_params.n);

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
    let media_dto = MediaAccessDto::new(id, MediaType::RecordImage, Some(n));

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
        .upload_images(MediaType::RecordImage, upload_dto)
        .await?;

    Ok(RestApiResponse::success(format!(
        "Successfully uploaded {uploaded_count} image(s)"
    )))
}

/// Serves idol media files by idol ID
///
/// This endpoint serves jpg images for idols based on the provided ID.
/// It first checks if the idol exists in the database, then looks for image files
/// in the directory named by the idol's name under the configured private assets directory.
#[utoipa::path(
    get,
    path = "/records/media/idol/id/{idol_id}",
    params(
        ("idol_id" = i64, Path, description = "The unique identifier for the idol"),
    ),
    responses(
        (status = 200, description = "Idol media file served successfully", content_type = "image/*"),
        (status = 404, description = "Idol not found or no media file exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Media"
)]
pub async fn serve_idol_media_by_id(
    State(state): State<AppState>,
    Path(idol_id): Path<i64>,
) -> Result<Response, AppError> {
    // First check if the idol exists in the database
    let idol = state
        .luna_service
        .idol_service()
        .get_idol_by_id(idol_id)
        .await?;

    // Use the idol's name as the media ID
    let media_dto = MediaAccessDto::new(idol.name, MediaType::IdolImage, None);

    state
        .luna_service
        .file_service()
        .serve_media_file(media_dto)
        .await
}

/// Serves idol media files by idol name
///
/// This endpoint serves jpg images for idols based on the provided name.
/// It first checks if the idol exists in the database, then looks for image files
/// in the directory named by the idol's name under the configured private assets directory.
#[utoipa::path(
    get,
    path = "/records/media/idol/name/{idol_name}",
    params(
        ("idol_name" = String, Path, description = "The name of the idol"),
    ),
    responses(
        (status = 200, description = "Idol media file served successfully", content_type = "image/*"),
        (status = 404, description = "Idol not found or no media file exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Media"
)]
pub async fn serve_idol_media_by_name(
    State(state): State<AppState>,
    Path(idol_name): Path<String>,
) -> Result<Response, AppError> {
    // First check if the idol exists in the database by searching by name
    use crate::domains::luna::dto::SearchIdolDto;
    let search_dto = SearchIdolDto {
        id: None,
        name: Some(idol_name.clone()),
        link: None,
        search: None,
    };

    let idols = state
        .luna_service
        .idol_service()
        .get_idol_list(search_dto)
        .await?;

    if idols.is_empty() {
        return Err(AppError::NotFound(format!(
            "Idol with name '{idol_name}' not found"
        )));
    }

    // Use the idol's name as the media ID
    let media_dto = MediaAccessDto::new(idol_name, MediaType::IdolImage, None);

    state
        .luna_service
        .file_service()
        .serve_media_file(media_dto)
        .await
}

/// Upload idol images by idol ID
///
/// This endpoint accepts multipart form data with image files and uploads them
/// to the private assets directory under the subdirectory named by the idol's name.
/// Only uploads files that don't already exist (no overwriting).
#[utoipa::path(
    post,
    path = "/records/media/upload_idol_by_id/{idol_id}/",
    params(
        ("idol_id" = i64, Path, description = "The unique identifier for the idol"),
    ),
    request_body(
        content = String,
        description = "Multipart form data with image files",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "Images already exist", body = String),
        (status = 400, description = "Bad request - invalid data or idol ID not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Media"
)]
pub async fn upload_idol_images_by_id(
    State(state): State<AppState>,
    Path(idol_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    // Check if the idol exists in the database
    let idol = state
        .luna_service
        .idol_service()
        .get_idol_by_id(idol_id)
        .await?;

    let mut images: Vec<ImageData> = Vec::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::ValidationError(format!("Failed to parse multipart form data: {e}"))
    })? {
        let field_name = field.name().unwrap_or("").to_owned();

        if field_name.starts_with("file") {
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_owned();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::ValidationError(format!("Failed to read file data: {e}")))?;

            // Use the idol's name as the filename (without extension)
            images.push(ImageData {
                name: idol.name.clone(),
                mime: content_type,
                bytes: data.to_vec(),
            });
        }
    }

    if images.is_empty() {
        return Err(AppError::ValidationError(
            "No image files provided".to_owned(),
        ));
    }

    let upload_dto = UploadImageDto {
        id: idol.name,
        files: images,
    };

    let uploaded_count = state
        .luna_service
        .file_service()
        .upload_images(MediaType::IdolImage, upload_dto)
        .await?;

    if uploaded_count == 0 {
        Ok(RestApiResponse::success_with_message(
            "Images already exist".to_owned(),
            "No new images uploaded".to_owned(),
        ))
    } else {
        Ok(RestApiResponse::success_with_message(
            format!("Successfully uploaded {uploaded_count} image(s)"),
            format!("Uploaded {uploaded_count} images"),
        ))
    }
}

/// Upload idol images by idol name
///
/// This endpoint accepts multipart form data with image files and uploads them
/// to the private assets directory under the subdirectory named by the idol's name.
/// Only uploads files that don't already exist (no overwriting).
#[utoipa::path(
    post,
    path = "/records/media/upload_idol_by_name/{idol_name}",
    params(
        ("idol_name" = String, Path, description = "The name of the idol"),
    ),
    request_body(
        content = String,
        description = "Multipart form data with image files",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "Images already exist", body = String),
        (status = 400, description = "Bad request - invalid data or idol name not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Media"
)]
pub async fn upload_idol_images_by_name(
    State(state): State<AppState>,
    Path(idol_name): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    // Check if the idol exists in the database by searching by name
    use crate::domains::luna::dto::SearchIdolDto;
    let search_dto = SearchIdolDto {
        id: None,
        name: Some(idol_name.clone()),
        link: None,
        search: None,
    };

    let idols = state
        .luna_service
        .idol_service()
        .get_idol_list(search_dto)
        .await?;

    if idols.is_empty() {
        return Err(AppError::ValidationError(format!(
            "Idol with name '{idol_name}' not found"
        )));
    }

    let mut images: Vec<ImageData> = Vec::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::ValidationError(format!("Failed to parse multipart form data: {e}"))
    })? {
        let field_name = field.name().unwrap_or("").to_owned();

        if field_name.starts_with("file") {
            // Use the idol's name as the filename (without extension)
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_owned();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::ValidationError(format!("Failed to read file data: {e}")))?;

            // Use the idol's name as the filename (without extension)
            images.push(ImageData {
                name: idol_name.clone(),
                mime: content_type,
                bytes: data.to_vec(),
            });
        }
    }

    if images.is_empty() {
        return Err(AppError::ValidationError(
            "No image files provided".to_owned(),
        ));
    }

    let upload_dto = UploadImageDto {
        id: idol_name,
        files: images,
    };

    let uploaded_count = state
        .luna_service
        .file_service()
        .upload_images(MediaType::IdolImage, upload_dto)
        .await?;

    if uploaded_count == 0 {
        Ok(RestApiResponse::success_with_message(
            "Images already exist".to_owned(),
            "No new images uploaded".to_owned(),
        ))
    } else {
        Ok(RestApiResponse::success_with_message(
            format!("Successfully uploaded {uploaded_count} image(s)"),
            format!("Uploaded {uploaded_count} images"),
        ))
    }
}
