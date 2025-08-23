use crate::common::{config::Config, error::AppError};
use crate::domains::luna::domain::FileServiceTrait;
use crate::domains::luna::dto::{MediaAccessDto, MediaType, UploadImageDto};
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
        // Build the file directory path: assets_private_path/records/images/id/
        let file_dir = Path::new(&self.config.assets_private_path)
            .join("images")
            .join(media_dto.media_type.get_sub_dir_name())
            .join(&media_dto.id);
        let filename_base = media_dto.get_filename();

        // Check if the directory exists
        if !file_dir.exists() {
            tracing::error!("Directory not found: {}", file_dir.display());
            return Err(AppError::NotFound(format!(
                "Media directory for id '{}' not found",
                media_dto.id
            )));
        }

        // Try to find the file with different extensions
        let supported_extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];
        let mut file_path = None;
        let mut found_extension = None;

        for extension in &supported_extensions {
            let candidate_path = file_dir.join(format!("{filename_base}.{extension}"));
            if candidate_path.exists() {
                file_path = Some(candidate_path);
                found_extension = Some(extension);
                break;
            }
        }

        let file_path = file_path.ok_or_else(|| {
            tracing::error!("No supported image file found for: {}", filename_base);
            AppError::NotFound(format!(
                "Media file '{filename_base}' not found with any supported extension"
            ))
        })?;

        // Read the file content
        let file_content = fs::read(&file_path).await.map_err(|err| {
            tracing::error!("Error reading file {}: {}", file_path.display(), err);
            AppError::InternalError
        })?;

        // Determine content type based on the found extension
        let content_type = Self::get_content_type_from_filename(found_extension.unwrap_or(&""));

        // Create the response with cache headers
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, file_content.len())
            .header(header::CACHE_CONTROL, "public, max-age=3600, immutable")
            .header(
                header::ETAG,
                format!("\"{:x}\"", md5::compute(&file_content)),
            )
            .body(Body::from(file_content))
            .map_err(|err| {
                tracing::error!("Error building response: {}", err);
                AppError::InternalError
            })?;

        Ok(response)
    }

    async fn upload_images(
        &self,
        ty: MediaType,
        upload_dto: UploadImageDto,
    ) -> Result<usize, AppError> {
        // Build the target directory path: assets_private_path/records/images/id/
        let target_dir = Path::new(&self.config.assets_private_path)
            .join("images")
            .join(ty.get_sub_dir_name())
            .join(&upload_dto.id);

        // Check if directory exists, create if not
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir).await.map_err(|err| {
                tracing::error!("Error creating directory {}: {}", target_dir.display(), err);
                AppError::InternalError
            })?;
        }

        let mut uploaded_count = 0;

        for image_data in upload_dto.files {
            // Generate filename based on name and mime type
            let extension = Self::get_extension_from_mime(&image_data.mime);
            let filename = format!("{}.{}", image_data.name, extension);
            let file_path = target_dir.join(&filename);

            // Check if file already exists, skip if it does (don't overwrite)
            if file_path.exists() {
                tracing::info!("File {} already exists, skipping", file_path.display());
                continue;
            }

            // Write the file
            match fs::write(&file_path, &image_data.bytes).await {
                Ok(_) => {
                    tracing::info!("Successfully uploaded file: {}", file_path.display());
                    uploaded_count += 1;
                }
                Err(err) => {
                    tracing::error!("Error writing file {}: {}", file_path.display(), err);
                    // Continue with other files instead of failing completely
                }
            }
        }

        Ok(uploaded_count)
    }
}

impl FileService {
    /// Get content type based on file extension
    fn get_content_type_from_filename(ext: &str) -> &'static str {
        match ext {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "svg" => "image/svg+xml",
            _ => "application/octet-stream",
        }
    }

    /// Get file extension from MIME type
    fn get_extension_from_mime(mime: &str) -> &'static str {
        match mime {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/bmp" => "bmp",
            "image/svg+xml" => "svg",
            _ => "bin",
        }
    }
}
