use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError},
    domains::luna::dto::{PaginatedResponse, PaginationQuery, RecordDto},
};

use axum::{extract::State, response::IntoResponse};

// Records by entity handlers
#[utoipa::path(
    get,
    path = "/cards/director-records",
    params(
        ("director_id" = i64, Query, description = "Director ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by director", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_director(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let director_id = params
        .get("director_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| AppError::ValidationError("director_id is required".into()))?;

    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_director(director_id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/studio-records",
    params(
        ("studio_id" = i64, Query, description = "Studio ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by studio", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_studio(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let studio_id = params
        .get("studio_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| AppError::ValidationError("studio_id is required".into()))?;

    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_studio(studio_id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/label-records",
    params(
        ("label_id" = i64, Query, description = "Label ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by label", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_label(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let label_id = params
        .get("label_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| AppError::ValidationError("label_id is required".into()))?;

    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_label(label_id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/series-records",
    params(
        ("series_id" = i64, Query, description = "Series ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by series", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_series(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let series_id = params
        .get("series_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| AppError::ValidationError("series_id is required".into()))?;

    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_series(series_id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/genre-records",
    params(
        ("genre_id" = i64, Query, description = "Genre ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by genre", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_genre(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let genre_id = params
        .get("genre_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| AppError::ValidationError("genre_id is required".into()))?;

    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_genre(genre_id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/idol-records",
    params(
        ("idol_id" = i64, Query, description = "Idol ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by idol", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_idol(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let idol_id = params
        .get("idol_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| AppError::ValidationError("idol_id is required".into()))?;

    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_idol(idol_id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}
