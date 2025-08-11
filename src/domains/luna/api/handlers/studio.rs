use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::{
        CreateStudioDto, PaginationQuery, SearchStudioDto, StudioDto, UpdateStudioDto,
    },
};

use axum::{extract::State, response::IntoResponse, Extension, Json};

use validator::Validate as _;

// Studio handlers
#[utoipa::path(
    get,
    path = "/cards/studios/{id}",
    responses((status = 200, description = "Get studio by ID", body = StudioDto)),
    tag = "Studios"
)]
pub async fn get_studio_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let studio = state
        .luna_service
        .studio_service()
        .get_studio_by_id(id)
        .await?;
    Ok(RestApiResponse::success(studio))
}

#[utoipa::path(
    get,
    path = "/cards/studios",
    responses((status = 200, description = "List all studios")),
    tag = "Studios"
)]
pub async fn get_studios(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search_dto = SearchStudioDto {
        id: None,
        name: None,
        link: None,
    };

    let paginated_result = state
        .luna_service
        .studio_service()
        .get_studio_list_paginated(search_dto, pagination)
        .await?;
    Ok(RestApiResponse::success(paginated_result))
}

#[utoipa::path(
    post,
    path = "/cards/studios",
    request_body = CreateStudioDto,
    responses((status = 201, description = "Studio created", body = StudioDto)),
    tag = "Studios"
)]
pub async fn create_studio(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<CreateStudioDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let studio = state
        .luna_service
        .studio_service()
        .create_studio(body)
        .await?;
    Ok(RestApiResponse::success(studio))
}

#[utoipa::path(
    put,
    path = "/cards/studios/{id}",
    request_body = UpdateStudioDto,
    responses((status = 200, description = "Studio updated", body = StudioDto)),
    tag = "Studios"
)]
pub async fn update_studio(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(body): Json<UpdateStudioDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let studio = state
        .luna_service
        .studio_service()
        .update_studio(id, body)
        .await?;
    Ok(RestApiResponse::success(studio))
}

#[utoipa::path(
    patch,
    path = "/cards/studios/{id}",
    request_body = UpdateStudioDto,
    responses((status = 200, description = "Studio partially updated", body = StudioDto)),
    tag = "Studios"
)]
pub async fn patch_studio(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(body): Json<UpdateStudioDto>,
) -> Result<impl IntoResponse, AppError> {
    let studio = state
        .luna_service
        .studio_service()
        .update_studio(id, body)
        .await?;
    Ok(RestApiResponse::success(studio))
}

#[utoipa::path(
    delete,
    path = "/cards/studios/{id}",
    responses((status = 204, description = "Studio deleted")),
    tag = "Studios"
)]
pub async fn delete_studio(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let message = state
        .luna_service
        .studio_service()
        .delete_studio(id)
        .await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}
