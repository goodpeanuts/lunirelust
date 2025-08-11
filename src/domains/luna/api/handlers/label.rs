use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::{
        CreateLabelDto, LabelDto, PaginationQuery, SearchLabelDto, UpdateLabelDto,
    },
};

use axum::{extract::State, response::IntoResponse, Extension, Json};

use validator::Validate as _;

// Label handlers
#[utoipa::path(
    get,
    path = "/cards/labels/{id}",
    responses((status = 200, description = "Get label by ID", body = LabelDto)),
    tag = "Labels"
)]
pub async fn get_label_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let label = state
        .luna_service
        .label_service()
        .get_label_by_id(id)
        .await?;
    Ok(RestApiResponse::success(label))
}

#[utoipa::path(
    get,
    path = "/cards/labels",
    responses((status = 200, description = "List all labels", body = [LabelDto])),
    tag = "Labels"
)]
pub async fn get_labels(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search_dto = SearchLabelDto {
        id: None,
        name: None,
        link: None,
    };

    // Always use paginated response for consistency
    let paginated_result = state
        .luna_service
        .label_service()
        .get_label_list_paginated(search_dto, pagination)
        .await?;
    Ok(RestApiResponse::success(paginated_result))
}

#[utoipa::path(
    post,
    path = "/cards/labels",
    request_body = CreateLabelDto,
    responses((status = 201, description = "Create a new label", body = LabelDto)),
    tag = "Labels"
)]
pub async fn create_label(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<CreateLabelDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let label = state
        .luna_service
        .label_service()
        .create_label(payload)
        .await?;
    Ok(RestApiResponse::success(label))
}

#[utoipa::path(
    put,
    path = "/cards/labels/{id}",
    request_body = UpdateLabelDto,
    responses((status = 200, description = "Update label", body = LabelDto)),
    tag = "Labels"
)]
pub async fn update_label(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateLabelDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let label = state
        .luna_service
        .label_service()
        .update_label(id, payload)
        .await?;
    Ok(RestApiResponse::success(label))
}

#[utoipa::path(
    patch,
    path = "/cards/labels/{id}",
    request_body = UpdateLabelDto,
    responses((status = 200, description = "Partially update label", body = LabelDto)),
    tag = "Labels"
)]
pub async fn patch_label(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateLabelDto>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let label = state
        .luna_service
        .label_service()
        .update_label(id, payload)
        .await?;
    Ok(RestApiResponse::success(label))
}

#[utoipa::path(
    delete,
    path = "/cards/labels/{id}",
    responses((status = 204, description = "Label deleted")),
    tag = "Labels"
)]
pub async fn delete_label(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let message = state.luna_service.label_service().delete_label(id).await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}
