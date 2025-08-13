use crate::common::error::AppError;
use crate::domains::luna::dto::MediaAccessDto;
use async_trait::async_trait;
use axum::response::Response;

/// Service trait for handling file-related operations in luna domain
#[async_trait]
pub trait FileServiceTrait: Send + Sync {
    /// Serves a media file based on the provided media access parameters
    /// Returns the file content as a response or an error if the file is not found
    async fn serve_media_file(&self, media_dto: MediaAccessDto) -> Result<Response, AppError>;
}
