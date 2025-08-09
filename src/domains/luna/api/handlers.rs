use crate::{
    common::{app_state::AppState, dto::RestApiResponse, error::AppError, jwt::Claims},
    domains::luna::dto::luna_dto::{
        CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateRecordDto,
        CreateSeriesDto, CreateStudioDto, DirectorDto, GenreDto, IdolDto, LabelDto,
        PaginationQuery, RecordDto, SearchDirectorDto, SearchGenreDto, SearchIdolDto,
        SearchLabelDto, SearchRecordDto, SearchSeriesDto, SearchStudioDto, SeriesDto, StudioDto,
        UpdateDirectorDto, UpdateGenreDto, UpdateIdolDto, UpdateLabelDto, UpdateRecordDto,
        UpdateSeriesDto, UpdateStudioDto,
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
    request_body = UpdateRecordDto,
    responses((status = 200, description = "Record updated", body = RecordDto)),
    tag = "Records"
)]
pub async fn update_record(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<UpdateRecordDto>,
) -> Result<impl IntoResponse, AppError> {
    body.validate().map_err(|err| {
        tracing::error!("Validation error: {err}");
        AppError::ValidationError(format!("Invalid input: {err}"))
    })?;

    let record = state
        .luna_service
        .record_service()
        .update_record(&id, body)
        .await?;
    Ok(RestApiResponse::success(record))
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
