use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::{
        luna::dto::{PaginatedResponse, PaginationQuery},
        user::dto::interaction_dto::{
            BatchStatusRequestDto, InteractionStatusDto, MarkViewedResponse, ToggleLikeResponse,
        },
    },
};

use axum::{extract::State, response::IntoResponse, Extension, Json};
use std::collections::HashMap;

#[utoipa::path(
    post,
    path = "/cards/records/user/{record_id}/like",
    responses(
        (status = 200, description = "Like toggled", body = ToggleLikeResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Records"
)]
pub async fn toggle_like(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(record_id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let liked = state
        .user_service
        .interaction_service()
        .toggle_like(&claims.sub, &record_id)
        .await?;
    Ok(RestApiResponse::success(ToggleLikeResponse { liked }))
}

#[utoipa::path(
    post,
    path = "/cards/records/user/{record_id}/viewed",
    responses(
        (status = 200, description = "Record marked as viewed", body = MarkViewedResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Records"
)]
pub async fn mark_viewed(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(record_id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state
        .user_service
        .interaction_service()
        .mark_viewed(&claims.sub, &record_id)
        .await?;
    Ok(RestApiResponse::success(MarkViewedResponse {
        viewed: true,
    }))
}

#[utoipa::path(
    post,
    path = "/cards/records/user/status",
    request_body = BatchStatusRequestDto,
    responses(
        (status = 200, description = "Batch interaction status", body = HashMap<String, InteractionStatusDto>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Records"
)]
pub async fn batch_status(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BatchStatusRequestDto>,
) -> Result<impl IntoResponse, AppError> {
    let status_map = state
        .user_service
        .interaction_service()
        .batch_get_status(&claims.sub, &body.record_ids)
        .await?;

    let results: HashMap<String, InteractionStatusDto> = body
        .record_ids
        .into_iter()
        .map(|id| {
            let status = status_map.get(&id).cloned().unwrap_or_default();
            (
                id,
                InteractionStatusDto {
                    liked: status.liked,
                    viewed: status.viewed,
                },
            )
        })
        .collect();

    Ok(RestApiResponse::success(results))
}

#[utoipa::path(
    get,
    path = "/cards/records/user/viewed",
    params(
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Viewed record IDs", body = PaginatedResponse<String>)),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Records"
)]
pub async fn get_viewed_record_ids(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let result = state
        .user_service
        .interaction_service()
        .get_viewed_record_ids_paginated(&claims.sub, pagination)
        .await?;
    Ok(RestApiResponse::success(result))
}
