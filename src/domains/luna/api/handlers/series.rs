use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::{
        CreateSeriesDto, PaginationQuery, SearchSeriesDto, SeriesDto, UpdateSeriesDto,
    },
};

use axum::{extract::State, response::IntoResponse, Extension, Json};

use validator::Validate as _;

// Series handlers
#[utoipa::path(
    get,
    path = "/cards/series/{id}",
    responses((status = 200, description = "Get series by ID", body = SeriesDto)),
    tag = "Series"
)]
pub async fn get_series_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let series = state
        .luna_service
        .series_service()
        .get_series_by_id(id)
        .await?;
    Ok(RestApiResponse::success(series))
}

#[utoipa::path(
    get,
    path = "/cards/series",
    responses((status = 200, description = "List all series")),
    tag = "Series"
)]
pub async fn get_series(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search_dto = SearchSeriesDto {
        id: None,
        name: None,
        link: None,
    };

    let paginated_result = state
        .luna_service
        .series_service()
        .get_series_list_paginated(search_dto, pagination)
        .await?;
    Ok(RestApiResponse::success(paginated_result))
}

#[utoipa::path(
    post,
    path = "/cards/series",
    request_body = CreateSeriesDto,
    responses((status = 201, description = "Series created", body = SeriesDto)),
    tag = "Series"
)]
pub async fn create_series(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<CreateSeriesDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let series = state
        .luna_service
        .series_service()
        .create_series(body)
        .await?;
    Ok(RestApiResponse::success(series))
}

#[utoipa::path(
    put,
    path = "/cards/series/{id}",
    request_body = UpdateSeriesDto,
    responses((status = 200, description = "Series updated", body = SeriesDto)),
    tag = "Series"
)]
pub async fn update_series(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(body): Json<UpdateSeriesDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let series = state
        .luna_service
        .series_service()
        .update_series(id, body)
        .await?;
    Ok(RestApiResponse::success(series))
}

#[utoipa::path(
    patch,
    path = "/cards/series/{id}",
    request_body = UpdateSeriesDto,
    responses((status = 200, description = "Series partially updated", body = SeriesDto)),
    tag = "Series"
)]
pub async fn patch_series(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(body): Json<UpdateSeriesDto>,
) -> Result<impl IntoResponse, AppError> {
    let series = state
        .luna_service
        .series_service()
        .update_series(id, body)
        .await?;
    Ok(RestApiResponse::success(series))
}

#[utoipa::path(
    delete,
    path = "/cards/series/{id}",
    responses((status = 204, description = "Series deleted")),
    tag = "Series"
)]
pub async fn delete_series(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let message = state
        .luna_service
        .series_service()
        .delete_series(id)
        .await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}
