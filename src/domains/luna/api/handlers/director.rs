use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::{
        CreateDirectorDto, DirectorDto, PaginationQuery, SearchDirectorDto, UpdateDirectorDto,
    },
};

use axum::{extract::State, response::IntoResponse, Extension, Json};

use validator::Validate as _;

// Director handlers
#[utoipa::path(
    get,
    path = "/cards/directors/{id}",
    responses((status = 200, description = "Get director by ID", body = DirectorDto)),
    tag = "Directors"
)]
pub async fn get_director_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let director = state
        .luna_service
        .director_service()
        .get_director_by_id(id)
        .await?;
    Ok(RestApiResponse::success(director))
}

#[utoipa::path(
    get,
    path = "/cards/directors",
    responses((status = 200, description = "List all directors", body = [DirectorDto])),
    tag = "Directors"
)]
pub async fn get_directors(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search_dto = SearchDirectorDto {
        id: None,
        name: None,
        link: None,
    };

    // Always use paginated response for consistency
    let paginated_result = state
        .luna_service
        .director_service()
        .get_director_list_paginated(search_dto, pagination)
        .await?;
    Ok(RestApiResponse::success(paginated_result))
}

#[utoipa::path(
    post,
    path = "/cards/directors",
    request_body = CreateDirectorDto,
    responses((status = 201, description = "Create a new director", body = DirectorDto)),
    tag = "Directors"
)]
pub async fn create_director(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<CreateDirectorDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let director = state
        .luna_service
        .director_service()
        .create_director(payload)
        .await?;
    Ok(RestApiResponse::success(director))
}

#[utoipa::path(
    put,
    path = "/cards/directors/{id}",
    request_body = UpdateDirectorDto,
    responses((status = 200, description = "Update director", body = DirectorDto)),
    tag = "Directors"
)]
pub async fn update_director(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateDirectorDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let director = state
        .luna_service
        .director_service()
        .update_director(id, payload)
        .await?;
    Ok(RestApiResponse::success(director))
}

#[utoipa::path(
    patch,
    path = "/cards/directors/{id}",
    request_body = UpdateDirectorDto,
    responses((status = 200, description = "Partially update director", body = DirectorDto)),
    tag = "Directors"
)]
pub async fn patch_director(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateDirectorDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let director = state
        .luna_service
        .director_service()
        .update_director(id, payload)
        .await?;
    Ok(RestApiResponse::success(director))
}

#[utoipa::path(
    delete,
    path = "/cards/directors/{id}",
    responses((status = 204, description = "Director deleted")),
    tag = "Directors"
)]
pub async fn delete_director(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let message = state
        .luna_service
        .director_service()
        .delete_director(id)
        .await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}
