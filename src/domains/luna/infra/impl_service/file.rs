use crate::common::{config::Config, error::AppError};
use crate::domains::luna::domain::FileServiceTrait;
use crate::domains::luna::dto::MediaAccessDto;
use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
};
use std::path::Path;
use tokio::fs;

/// Implementation of the file service for luna domain
#[derive(Clone)]
pub struct FileService {
    config: Config,
}

impl FileService {
    /// Creates a new `FileService` instance
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl FileServiceTrait for FileService {
    /// Serves a media file based on the provided media access parameters
    async fn serve_media_file(&self, media_dto: MediaAccessDto) -> Result<Response, AppError> {
        // Build the file path: assets_private_path/id/filename
        let file_dir = Path::new(&self.config.assets_private_path)
            .join("images")
            .join(&media_dto.id);
        let filename = media_dto.get_filename();
        let file_path = file_dir.join(&filename);

        // Check if the directory exists
        if !file_dir.exists() {
            tracing::error!("Directory not found: {}", file_dir.display());
            return Err(AppError::NotFound(format!(
                "Media directory for id '{}' not found",
                media_dto.id
            )));
        }

        // Check if the file exists
        if !file_path.exists() {
            tracing::error!("File not found: {}", file_path.display());
            return Err(AppError::NotFound(format!(
                "Media file '{filename}' not found"
            )));
        }

        // Read the file content
        let file_content = fs::read(&file_path).await.map_err(|err| {
            tracing::error!("Error reading file {}: {}", file_path.display(), err);
            AppError::InternalError
        })?;

        // Determine content type based on file extension
        let content_type = "image/jpg"; // Since we're only serving jpg files

        // Create the response
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, file_content.len())
            .body(Body::from(file_content))
            .map_err(|err| {
                tracing::error!("Error building response: {}", err);
                AppError::InternalError
            })?;

        Ok(response)
    }
}
