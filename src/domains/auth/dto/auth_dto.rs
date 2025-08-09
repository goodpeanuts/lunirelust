use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RegisterDto {
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,
    #[validate(email(message = "Email is invalid"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
}
