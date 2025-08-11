use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::{
        CreateGenreDto, GenreDto, PaginationQuery, SearchGenreDto, UpdateGenreDto,
    },
};

use axum::{extract::State, response::IntoResponse, Extension, Json};

use validator::Validate as _;

// Genre handlers
#[utoipa::path(
    get,
    path = "/cards/genres/{id}",
    responses((status = 200, description = "Get genre by ID", body = GenreDto)),
    tag = "Genres"
)]
pub async fn get_genre_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let genre = state
        .luna_service
        .genre_service()
        .get_genre_by_id(id)
        .await?;
    Ok(RestApiResponse::success(genre))
}

#[utoipa::path(
    get,
    path = "/cards/genres",
    responses((status = 200, description = "List all genres", body = [GenreDto])),
    tag = "Genres"
)]
pub async fn get_genres(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search_dto = SearchGenreDto {
        id: None,
        name: None,
        link: None,
    };

    // Always use paginated response for consistency
    let paginated_result = state
        .luna_service
        .genre_service()
        .get_genre_list_paginated(search_dto, pagination)
        .await?;
    Ok(RestApiResponse::success(paginated_result))
}

#[utoipa::path(
    post,
    path = "/cards/genres",
    request_body = CreateGenreDto,
    responses((status = 201, description = "Create a new genre", body = GenreDto)),
    tag = "Genres"
)]
pub async fn create_genre(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<CreateGenreDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let genre = state
        .luna_service
        .genre_service()
        .create_genre(payload)
        .await?;
    Ok(RestApiResponse::success(genre))
}

#[utoipa::path(
    put,
    path = "/cards/genres/{id}",
    request_body = UpdateGenreDto,
    responses((status = 200, description = "Update genre", body = GenreDto)),
    tag = "Genres"
)]
pub async fn update_genre(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateGenreDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let genre = state
        .luna_service
        .genre_service()
        .update_genre(id, payload)
        .await?;
    Ok(RestApiResponse::success(genre))
}

#[utoipa::path(
    patch,
    path = "/cards/genres/{id}",
    request_body = UpdateGenreDto,
    responses((status = 200, description = "Partially update genre", body = GenreDto)),
    tag = "Genres"
)]
pub async fn patch_genre(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateGenreDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let genre = state
        .luna_service
        .genre_service()
        .update_genre(id, payload)
        .await?;
    Ok(RestApiResponse::success(genre))
}

#[utoipa::path(
    delete,
    path = "/cards/genres/{id}",
    responses((status = 204, description = "Genre deleted")),
    tag = "Genres"
)]
pub async fn delete_genre(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let message = state.luna_service.genre_service().delete_genre(id).await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}
