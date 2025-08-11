use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError},
    domains::luna::dto::EntityCountDto,
};

use axum::{extract::State, response::IntoResponse};

// Count handlers
#[utoipa::path(
    get,
    path = "/cards/director-records-count",
    responses((status = 200, description = "Get director record counts", body = [EntityCountDto])),
    tag = "Statistics"
)]
pub async fn get_director_records_count(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let counts = state
        .luna_service
        .director_service()
        .get_director_record_counts()
        .await?;
    Ok(RestApiResponse::success(counts))
}

#[utoipa::path(
    get,
    path = "/cards/genre-records-count",
    responses((status = 200, description = "Get genre record counts", body = [EntityCountDto])),
    tag = "Statistics"
)]
pub async fn get_genre_records_count(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let counts = state
        .luna_service
        .genre_service()
        .get_genre_record_counts()
        .await?;
    Ok(RestApiResponse::success(counts))
}

#[utoipa::path(
    get,
    path = "/cards/label-records-count",
    responses((status = 200, description = "Get label record counts", body = [EntityCountDto])),
    tag = "Statistics"
)]
pub async fn get_label_records_count(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let counts = state
        .luna_service
        .label_service()
        .get_label_record_counts()
        .await?;
    Ok(RestApiResponse::success(counts))
}

#[utoipa::path(
    get,
    path = "/cards/studio-records-count",
    responses((status = 200, description = "Get studio record counts", body = [EntityCountDto])),
    tag = "Statistics"
)]
pub async fn get_studio_records_count(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let counts = state
        .luna_service
        .studio_service()
        .get_studio_record_counts()
        .await?;
    Ok(RestApiResponse::success(counts))
}

#[utoipa::path(
    get,
    path = "/cards/series-records-count",
    responses((status = 200, description = "Get series record counts", body = [EntityCountDto])),
    tag = "Statistics"
)]
pub async fn get_series_records_count(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let counts = state
        .luna_service
        .series_service()
        .get_series_record_counts()
        .await?;
    Ok(RestApiResponse::success(counts))
}

#[utoipa::path(
    get,
    path = "/cards/idol-records-count",
    responses((status = 200, description = "Get idol record counts", body = [EntityCountDto])),
    tag = "Statistics"
)]
pub async fn get_idol_records_count(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let counts = state
        .luna_service
        .idol_service()
        .get_idol_record_counts()
        .await?;
    Ok(RestApiResponse::success(counts))
}
