use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    BoxError,
};

use sea_orm::DbErr as DbError;
use thiserror::Error;
use tracing::error;

use crate::common::dto::RestApiResponse;

use super::dto::ApiResponse;

/// `AppError` is an enum that represents various types of errors that can occur in the application.
/// It implements the `std::error::Error` trait and the `axum::response::IntoResponse` trait.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbError), // Used for database-related errors

    #[error("Not found: {0}")]
    NotFound(String), // Used for not found errors

    #[error("Internal server error")]
    InternalError,

    #[error("Internal server error: {0}")]
    InternalErrorWithMessage(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Forbidden Request")]
    Forbidden,

    /// Used for file-related errors
    #[error("File data is empty")]
    InvalidFileData,

    #[error("File size exceeded")]
    FileSizeExceeded,

    #[error("Invalid file name")]
    InvalidFileName,

    #[error("Unsupported file extension")]
    UnsupportedFileExtension,

    /// Used for authentication-related errors
    #[error("Wrong credentials")]
    WrongCredentials,
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token creation error")]
    TokenCreation,
    #[error("User not found")]
    UserNotFound,
}

/// Converts the `AppError` enum into an HTTP response.
/// It maps the error to an appropriate HTTP status code and constructs a JSON response body.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::ValidationError(_)
            | Self::InvalidFileData
            | Self::FileSizeExceeded
            | Self::InvalidFileName
            | Self::UnsupportedFileExtension
            | Self::MissingCredentials => StatusCode::BAD_REQUEST,
            Self::DatabaseError(_)
            | Self::InternalError
            | Self::InternalErrorWithMessage(_)
            | Self::TokenCreation => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound(_) | Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::WrongCredentials | Self::InvalidToken => StatusCode::UNAUTHORIZED,
        };
        let body = axum::Json(ApiResponse::<()> {
            status: status.as_u16(),
            message: self.to_string(),
            data: None,
        });

        (status, body).into_response()
    }
}

/// `handle_error` is a function that middlewares the error handling in the application.
///
/// It takes a `BoxError` as input and returns an HTTP response.
/// It maps the error to an appropriate HTTP status code and constructs a JSON response body.
/// The function is used to handle errors that occur during the request processing.
/// It is designed to be used with the axum framework.
pub async fn handle_error(error: BoxError) -> impl IntoResponse {
    let status = if error.is::<tower::timeout::error::Elapsed>() {
        StatusCode::REQUEST_TIMEOUT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    let message = error.to_string();
    error!(?status, %message, "Request failed");

    let body = RestApiResponse::<()>::failure(status.as_u16(), message);

    (status, body)
}
