use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::{
        CreateIdolDto, IdolDto, IdolWithoutImageDto, PaginationQuery, SearchIdolDto, UpdateIdolDto,
    },
};

use axum::{extract::State, response::IntoResponse, Extension, Json};

use validator::Validate as _;

// Idol handlers
#[utoipa::path(
    get,
    path = "/cards/idols/{id}",
    responses((status = 200, description = "Get idol by ID", body = IdolDto)),
    tag = "Idols"
)]
pub async fn get_idol_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let idol = state.luna_service.idol_service().get_idol_by_id(id).await?;
    Ok(RestApiResponse::success(idol))
}

#[utoipa::path(
    get,
    path = "/cards/idols",
    responses((status = 200, description = "List all idols")),
    tag = "Idols"
)]
pub async fn get_idols(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search_dto = SearchIdolDto {
        id: None,
        name: None,
        link: None,
        search: None,
    };

    let paginated_result = state
        .luna_service
        .idol_service()
        .get_idol_list_paginated(search_dto, pagination)
        .await?;
    Ok(RestApiResponse::success(paginated_result))
}

#[utoipa::path(
    post,
    path = "/cards/idols",
    request_body = CreateIdolDto,
    responses((status = 201, description = "Idol created", body = IdolDto)),
    tag = "Idols"
)]
pub async fn create_idol(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<CreateIdolDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let idol = state.luna_service.idol_service().create_idol(body).await?;
    Ok(RestApiResponse::success(idol))
}

#[utoipa::path(
    put,
    path = "/cards/idols/{id}",
    request_body = UpdateIdolDto,
    responses((status = 200, description = "Idol updated", body = IdolDto)),
    tag = "Idols"
)]
pub async fn update_idol(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(body): Json<UpdateIdolDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let idol = state
        .luna_service
        .idol_service()
        .update_idol(id, body)
        .await?;
    Ok(RestApiResponse::success(idol))
}

#[utoipa::path(
    patch,
    path = "/cards/idols/{id}",
    request_body = UpdateIdolDto,
    responses((status = 200, description = "Idol partially updated", body = IdolDto)),
    tag = "Idols"
)]
pub async fn patch_idol(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(body): Json<UpdateIdolDto>,
) -> Result<impl IntoResponse, AppError> {
    let idol = state
        .luna_service
        .idol_service()
        .update_idol(id, body)
        .await?;
    Ok(RestApiResponse::success(idol))
}

#[utoipa::path(
    delete,
    path = "/cards/idols/{id}",
    responses((status = 204, description = "Idol deleted")),
    tag = "Idols"
)]
pub async fn delete_idol(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let message = state.luna_service.idol_service().delete_idol(id).await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}

/// Get idols without images
///
/// This endpoint returns a list of idols that don't have any images
/// in their media directory.
#[utoipa::path(
    get,
    path = "/cards/idols/without-images",
    responses((status = 200, description = "Get idols without images", body = Vec<IdolWithoutImageDto>)),
    tag = "Idols"
)]
pub async fn get_idols_without_images(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let idols_without_images = state
        .luna_service
        .idol_service()
        .get_idols_without_images(&state.config.assets_private_path)
        .await?;
    Ok(RestApiResponse::success(idols_without_images))
}
