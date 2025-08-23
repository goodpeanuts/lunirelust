use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::{
        CreateLinkDto, CreateRecordDto, PaginatedResponse, PaginationQuery, RecordDto,
        RecordSlimDto, SearchRecordDto, UpdateRecordDto,
    },
};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};

use validator::Validate as _;

// Record handlers
#[utoipa::path(
    get,
    path = "/cards/records/{id}",
    responses((status = 200, description = "Get record by ID", body = RecordDto)),
    tag = "Records"
)]
pub async fn get_record_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .luna_service
        .record_service()
        .get_record_by_id(&id)
        .await?;
    Ok(RestApiResponse::success(record))
}

#[utoipa::path(
    get,
    path = "/cards/records",
    responses((status = 200, description = "List all records")),
    tag = "Records"
)]
pub async fn get_records(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let search_dto = SearchRecordDto {
        id: None,
        title: None,
        director_id: None,
        studio_id: None,
        label_id: None,
        series_id: None,
        search: None,
    };

    let paginated_result = state
        .luna_service
        .record_service()
        .get_record_list_paginated(search_dto, pagination)
        .await?;
    Ok(RestApiResponse::success(paginated_result))
}

#[utoipa::path(
    post,
    path = "/cards/records",
    request_body = CreateRecordDto,
    responses((status = 201, description = "Record created", body = RecordDto)),
    tag = "Records"
)]
pub async fn create_record(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<CreateRecordDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let record = state
        .luna_service
        .record_service()
        .create_record(body)
        .await?;
    Ok(RestApiResponse::success(record))
}

#[utoipa::path(
    put,
    path = "/cards/records/{id}",
    request_body = Vec<CreateLinkDto>,
    responses(
        (status = 200, description = "No new links added", body = i32),
        (status = 201, description = "New links added successfully", body = i32)
    ),
    tag = "Records"
)]
pub async fn update_record(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Vec<CreateLinkDto>>,
) -> Result<impl IntoResponse, AppError> {
    let added_count = state
        .luna_service
        .record_service()
        .update_record_links(&id, body)
        .await?;

    if added_count > 0 {
        Ok((StatusCode::CREATED, RestApiResponse::success(added_count)))
    } else {
        Ok((StatusCode::OK, RestApiResponse::success(added_count)))
    }
}

#[utoipa::path(
    patch,
    path = "/cards/records/{id}",
    request_body = UpdateRecordDto,
    responses((status = 200, description = "Record partially updated", body = RecordDto)),
    tag = "Records"
)]
pub async fn patch_record(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<UpdateRecordDto>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .luna_service
        .record_service()
        .update_record(&id, body)
        .await?;
    Ok(RestApiResponse::success(record))
}

#[utoipa::path(
    patch,
    path = "/cards/records/{id}/links",
    request_body = Vec<CreateLinkDto>,
    responses((status = 200, description = "Record links updated", body = i32)),
    tag = "Records"
)]
pub async fn update_record_links(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<Vec<CreateLinkDto>>,
) -> Result<impl IntoResponse, AppError> {
    let added_count = state
        .luna_service
        .record_service()
        .update_record_links(&id, body)
        .await?;
    Ok(RestApiResponse::success(added_count))
}

#[utoipa::path(
    delete,
    path = "/cards/records/{id}",
    responses((status = 204, description = "Record deleted")),
    tag = "Records"
)]
pub async fn delete_record(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let message = state
        .luna_service
        .record_service()
        .delete_record(&id)
        .await?;
    Ok(RestApiResponse::success_with_message(message, ()))
}

// Records by entity handlers
#[utoipa::path(
    get,
    path = "/cards/director/{id}/records",
    params(
        ("id" = i64, Path, description = "Director ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by director", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_director(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_director(id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/studio/{id}/records",
    params(
        ("id" = i64, Path, description = "Studio ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by studio", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_studio(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_studio(id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/label/{id}/records",
    params(
        ("id" = i64, Path, description = "Label ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by label", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_label(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_label(id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/series/{id}/records",
    params(
        ("id" = i64, Path, description = "Series ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by series", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_series(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_series(id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/genre/{id}/records",
    params(
        ("id" = i64, Path, description = "Genre ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by genre", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_genre(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_genre(id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/idol/{id}/records",
    params(
        ("id" = i64, Path, description = "Idol ID"),
        ("limit" = Option<i64>, Query, description = "Limit for pagination"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses((status = 200, description = "Get records by idol", body = PaginatedResponse<RecordDto>)),
    tag = "Records"
)]
pub async fn get_records_by_idol(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok());
    let offset = params.get("offset").and_then(|s| s.parse::<i64>().ok());

    let pagination = PaginationQuery { limit, offset };

    let records = state
        .luna_service
        .record_service()
        .get_records_by_idol(id, pagination)
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/records/slim",
    responses((status = 200, description = "Get all record slim data", body = Vec<RecordSlimDto>)),
    tag = "Records"
)]
pub async fn get_all_record_slim(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let records = state
        .luna_service
        .record_service()
        .get_all_record_slim()
        .await?;
    Ok(RestApiResponse::success(records))
}

#[utoipa::path(
    get,
    path = "/cards/records/ids",
    params(
        ("limit" = Option<i64>, Query, description = "Limit for pagination (optional)"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination (optional)")
    ),
    responses((status = 200, description = "Get all record IDs with optional pagination", body = Vec<String>)),
    tag = "Records"
)]
pub async fn get_all_record_ids(
    State(state): State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let all_ids = state
        .luna_service
        .record_service()
        .get_all_record_ids()
        .await?;

    // Handle pagination - if no pagination params provided, return all IDs
    if pagination.limit.is_none() && pagination.offset.is_none() {
        Ok(RestApiResponse::success(all_ids))
    } else {
        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let paginated_ids: Vec<String> = all_ids.into_iter().skip(offset).take(limit).collect();

        Ok(RestApiResponse::success(paginated_ids))
    }
}
