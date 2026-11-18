use std::collections::HashMap;

use crate::{common::error::AppError, domains::file::FileDto};

// Retain the existing multipart metadata policy at its bounded parsing boundary.
static FORBIDDEN_PATTERN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"(?i)<\s*script\b[^>]*>").expect("valid multipart metadata pattern")
});

const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";

/// Parse validated text and file fields. Repeated text keeps the last value;
/// repeated file fields retain every file in arrival order.
pub async fn parse_multipart_to_maps(
    mut multipart: axum::extract::Multipart,
    asset_allowed_extensions_pattern: &regex::Regex,
) -> Result<(HashMap<String, String>, HashMap<String, Vec<FileDto>>), AppError> {
    let mut fields: HashMap<String, String> = HashMap::new();
    let mut files: HashMap<String, Vec<FileDto>> = HashMap::new();

    // Helper closure to map multipart errors to AppError.
    let map_err_internal = |err: axum::extract::multipart::MultipartError| {
        tracing::error!("Multipart error: {err}");
        AppError::InternalErrorWithMessage(err.to_string())
    };

    while let Some(field) = multipart.next_field().await.map_err(map_err_internal)? {
        let name = field
            .name()
            .ok_or_else(|| AppError::ValidationError("Field name is missing".to_owned()))?
            .to_owned();

        if FORBIDDEN_PATTERN.is_match(&name) {
            tracing::error!("Invalid field name: {}", name);
            return Err(AppError::Forbidden);
        }

        if let Some(filename) = field.file_name() {
            let original_filename = filename.to_owned();
            if original_filename.contains("..") || original_filename.contains('/') {
                tracing::error!("Invalid file name: {}", original_filename);
                return Err(AppError::InvalidFileName);
            }
            if FORBIDDEN_PATTERN.is_match(&original_filename) {
                tracing::error!("Invalid file name: {}", original_filename);
                return Err(AppError::Forbidden);
            }
            if !asset_allowed_extensions_pattern.is_match(&original_filename) {
                tracing::error!("Unsupported file extension: {}", original_filename);
                return Err(AppError::UnsupportedFileExtension);
            }
            let content_type = field
                .content_type()
                .unwrap_or(APPLICATION_OCTET_STREAM)
                .to_owned();
            let data = field.bytes().await.map_err(map_err_internal)?.to_vec();
            files.entry(name).or_default().push(FileDto {
                content_type,
                original_filename,
                data,
            });
        } else {
            let text = field.text().await.map_err(map_err_internal)?;
            if FORBIDDEN_PATTERN.is_match(&text) {
                tracing::error!("Invalid text field value: {}", text);
                return Err(AppError::Forbidden);
            }
            fields.insert(name, text);
        }
    }

    Ok((fields, files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, extract::FromRequest as _, http::Request};

    async fn parse(
        parts: &[(&str, Option<&str>, &str)],
    ) -> Result<(HashMap<String, String>, HashMap<String, Vec<FileDto>>), AppError> {
        let mut body = String::new();
        for (name, filename, value) in parts {
            body.push_str(&format!(
                "--fixture\r\nContent-Disposition: form-data; name=\"{name}\""
            ));
            if let Some(filename) = filename {
                body.push_str(&format!(
                    "; filename=\"{filename}\"\r\nContent-Type: image/png"
                ));
            }
            body.push_str(&format!("\r\n\r\n{value}\r\n"));
        }
        body.push_str("--fixture--\r\n");
        let request = Request::builder()
            .header("content-type", "multipart/form-data; boundary=fixture")
            .body(Body::from(body))
            .expect("valid multipart request");
        let multipart = axum::extract::Multipart::from_request(request, &())
            .await
            .expect("valid multipart boundary");
        parse_multipart_to_maps(
            multipart,
            &regex::Regex::new(r"\.(png|jpg)$").expect("valid extension pattern"),
        )
        .await
    }

    #[tokio::test]
    async fn repeated_text_keeps_last_value_and_repeated_files_keep_all_bytes() {
        let (fields, files) = parse(&[
            ("title", None, "first"),
            ("files", Some("first.png"), "first image"),
            ("title", None, "last"),
            ("files", Some("second.png"), "second image"),
        ])
        .await
        .expect("valid upload");
        assert_eq!(fields.get("title").map(String::as_str), Some("last"));
        let uploads = files.get("files").expect("uploaded files");
        assert_eq!(uploads.len(), 2);
        assert_eq!(uploads[0].original_filename, "first.png");
        assert_eq!(uploads[0].data, b"first image");
        assert_eq!(uploads[1].original_filename, "second.png");
        assert_eq!(uploads[1].data, b"second image");
    }

    #[tokio::test]
    async fn rejected_earlier_fields_are_not_hidden_by_later_values() {
        for rejected in [
            ("title", None, "<script>invalid</script>"),
            ("files", Some("../invalid.png"), "image"),
            ("files", Some("invalid.exe"), "image"),
        ] {
            assert!(parse(&[rejected, ("title", None, "valid")]).await.is_err());
        }
    }
}
