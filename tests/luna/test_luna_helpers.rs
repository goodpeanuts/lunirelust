#![allow(clippy::all)]
#![allow(dead_code)]

use axum::body::Body;
use axum::http::{Response, StatusCode};
use serde::{Deserialize, Serialize};

use lunirelust::{
    common::{dto::RestApiResponse, error::AppError},
    domains::luna::dto::{
        DirectorDto, GenreDto, IdolDto, LabelDto,
        PaginatedResponse, SeriesDto, StudioDto,
    },
};

// We'll import the test helper functions inline for each test file instead
// This avoids module dependency issues

/// Test data builder for creating test entities with proper dependencies
pub struct TestDataBuilder {
    pub director: Option<DirectorDto>,
    pub studio: Option<StudioDto>,
    pub label: Option<LabelDto>,
    pub series: Option<SeriesDto>,
    pub genre: Option<GenreDto>,
    pub idol: Option<IdolDto>,
}

impl TestDataBuilder {
    pub fn new() -> Self {
        Self {
            director: None,
            studio: None,
            label: None,
            series: None,
            genre: None,
            idol: None,
        }
    }
}

impl Default for TestDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a well-formatted JSON request body from any serializable struct
pub fn build_luna_json_request<T: Serialize>(payload: &T) -> Result<String, AppError> {
    serde_json::to_string_pretty(payload).map_err(|e| {
        AppError::InternalErrorWithMessage(format!("Failed to serialize payload: {e}"))
    })
}

/// Assert success response and extract data
pub async fn assert_success_response<T>(
    response: Response<Body>,
    validator: impl FnOnce(&T),
    deserialize_json_body: impl Fn(
        Body,
    ) -> Result<
        RestApiResponse<T>,
        Box<dyn std::error::Error + Send + Sync>,
    >,
) -> Result<T, AppError>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected status to be OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<T> = deserialize_json_body(body).map_err(|e| {
        AppError::InternalErrorWithMessage(format!("Failed to deserialize response: {e}"))
    })?;

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body
        .0
        .data
        .ok_or_else(|| AppError::InternalErrorWithMessage("No data in response".to_owned()))?;

    validator(&data);
    Ok(data)
}

/// Assert error response with specific status and message
pub async fn assert_error_response(
    response: Response<Body>,
    expected_status: StatusCode,
    expected_message_contains: &str,
) {
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status, expected_status,
        "Expected status {}, got {}",
        expected_status, parts.status
    );

    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .expect("Failed to read response body");
    let body_str = String::from_utf8_lossy(&body_bytes);

    assert!(
        body_str.contains(expected_message_contains),
        "Expected error message to contain '{}', got: {}",
        expected_message_contains,
        body_str
    );
}

/// Assert paginated response structure
pub async fn assert_paginated_response<T>(
    response: Response<Body>,
    validator: impl FnOnce(&PaginatedResponse<T>),
    deserialize_json_body: impl Fn(
        Body,
    ) -> Result<
        RestApiResponse<PaginatedResponse<T>>,
        Box<dyn std::error::Error + Send + Sync>,
    >,
) -> Result<PaginatedResponse<T>, AppError>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected status to be OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<PaginatedResponse<T>> = deserialize_json_body(body)
        .map_err(|e| {
            AppError::InternalErrorWithMessage(format!("Failed to deserialize response: {e}"))
        })?;

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body
        .0
        .data
        .ok_or_else(|| AppError::InternalErrorWithMessage("No data in response".to_owned()))?;

    validator(&data);
    Ok(data)
}
