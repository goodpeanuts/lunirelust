use super::handlers::{
    // Director handlers
    __path_create_director,
    // Genre handlers
    __path_create_genre,
    // Idol handlers
    __path_create_idol,
    // Label handlers
    __path_create_label,
    // Record handlers
    __path_create_record,
    // Series handlers
    __path_create_series,
    // Studio handlers
    __path_create_studio,
    __path_delete_director,
    __path_delete_genre,
    __path_delete_idol,
    __path_delete_label,
    __path_delete_record,
    __path_delete_series,
    __path_delete_studio,
    __path_get_director_by_id,
    // Auto-generated paths for count handlers
    __path_get_director_records_count,
    __path_get_directors,
    __path_get_genre_by_id,
    __path_get_genre_records_count,
    __path_get_genres,
    __path_get_idol_by_id,
    __path_get_idol_records_count,
    __path_get_idols,
    __path_get_label_by_id,
    __path_get_label_records_count,
    __path_get_labels,
    __path_get_record_by_id,
    __path_get_records,
    // Auto-generated paths for records by entity handlers
    __path_get_records_by_director,
    __path_get_records_by_genre,
    __path_get_records_by_idol,
    __path_get_records_by_label,
    __path_get_records_by_series,
    __path_get_records_by_studio,
    __path_get_series,
    __path_get_series_by_id,
    __path_get_series_records_count,
    __path_get_studio_by_id,
    __path_get_studio_records_count,
    __path_get_studios,
    __path_patch_director,
    __path_patch_genre,
    __path_patch_idol,
    __path_patch_label,
    __path_patch_record,
    __path_patch_series,
    __path_patch_studio,
    __path_update_director,
    __path_update_genre,
    __path_update_idol,
    __path_update_label,
    __path_update_record,
    __path_update_series,
    __path_update_studio,
    create_director,
    create_genre,
    create_idol,
    create_label,
    create_record,
    create_series,
    create_studio,
    delete_director,
    delete_genre,
    delete_idol,
    delete_label,
    delete_record,
    delete_series,
    delete_studio,
    get_director_by_id,
    // Count handlers
    get_director_records_count,
    get_directors,
    get_genre_by_id,
    get_genre_records_count,
    get_genres,
    get_idol_by_id,
    get_idol_records_count,
    get_idols,
    get_label_by_id,
    get_label_records_count,
    get_labels,
    get_record_by_id,
    get_records,
    // Records by entity handlers
    get_records_by_director,
    get_records_by_genre,
    get_records_by_idol,
    get_records_by_label,
    get_records_by_series,
    get_records_by_studio,
    get_series,
    get_series_by_id,
    get_series_records_count,
    get_studio_by_id,
    get_studio_records_count,
    get_studios,
    patch_director,
    patch_genre,
    patch_idol,
    patch_label,
    patch_record,
    patch_series,
    patch_studio,
    update_director,
    update_genre,
    update_idol,
    update_label,
    update_record,
    update_series,
    update_studio,
};

use crate::{
    common::app_state::AppState,
    domains::luna::dto::{
        CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateRecordDto,
        CreateSeriesDto, CreateStudioDto, DirectorDto, GenreDto, IdolDto, LabelDto, RecordDto,
        SeriesDto, StudioDto, UpdateDirectorDto, UpdateGenreDto, UpdateIdolDto, UpdateLabelDto,
        UpdateRecordDto, UpdateSeriesDto, UpdateStudioDto,
    },
};

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Director endpoints
        get_director_by_id,
        get_directors,
        create_director,
        update_director,
        patch_director,
        delete_director,
        // Genre endpoints
        get_genre_by_id,
        get_genres,
        create_genre,
        update_genre,
        patch_genre,
        delete_genre,
        // Label endpoints
        get_label_by_id,
        get_labels,
        create_label,
        update_label,
        patch_label,
        delete_label,
        // Studio endpoints
        get_studio_by_id,
        get_studios,
        create_studio,
        update_studio,
        patch_studio,
        delete_studio,
        // Series endpoints
        get_series_by_id,
        get_series,
        create_series,
        update_series,
        patch_series,
        delete_series,
        // Idol endpoints
        get_idol_by_id,
        get_idols,
        create_idol,
        update_idol,
        patch_idol,
        delete_idol,
        // Record endpoints
        get_record_by_id,
        get_records,
        create_record,
        update_record,
        patch_record,
        delete_record,
        // Count endpoints
        get_director_records_count,
        get_genre_records_count,
        get_label_records_count,
        get_studio_records_count,
        get_series_records_count,
        get_idol_records_count,
        // Records by entity endpoints
        get_records_by_director,
        get_records_by_studio,
        get_records_by_label,
        get_records_by_series,
        get_records_by_genre,
        get_records_by_idol,
    ),
    components(schemas(
        DirectorDto, CreateDirectorDto, UpdateDirectorDto,
        GenreDto, CreateGenreDto, UpdateGenreDto,
        LabelDto, CreateLabelDto, UpdateLabelDto,
        StudioDto, CreateStudioDto, UpdateStudioDto,
        SeriesDto, CreateSeriesDto, UpdateSeriesDto,
        IdolDto, CreateIdolDto, UpdateIdolDto,
        RecordDto, CreateRecordDto, UpdateRecordDto
    )),
    tags(
        (name = "Directors", description = "Director management endpoints"),
        (name = "Genres", description = "Genre management endpoints"),
        (name = "Labels", description = "Label management endpoints"),
        (name = "Studios", description = "Studio management endpoints"),
        (name = "Series", description = "Series management endpoints"),
        (name = "Idols", description = "Idol management endpoints"),
        (name = "Records", description = "Record management endpoints"),
        (name = "Statistics", description = "Statistics and count endpoints")
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&LunaApiDoc)
)]
/// This struct is used to generate `OpenAPI` documentation for the luna routes.
pub struct LunaApiDoc;

impl utoipa::Modify for LunaApiDoc {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .as_mut()
            .expect("Failed to parse environment variable");
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Input your `<your‑jwt>`"))
                    .build(),
            ),
        );
    }
}

pub fn luna_routes() -> Router<AppState> {
    Router::new()
        // Director routes
        .route("/directors", get(get_directors))
        .route("/directors", post(create_director))
        .route("/directors/{id}", get(get_director_by_id))
        .route("/directors/{id}", put(update_director))
        .route("/directors/{id}", patch(patch_director))
        .route("/directors/{id}", delete(delete_director))
        // Genre routes
        .route("/genres", get(get_genres))
        .route("/genres", post(create_genre))
        .route("/genres/{id}", get(get_genre_by_id))
        .route("/genres/{id}", put(update_genre))
        .route("/genres/{id}", patch(patch_genre))
        .route("/genres/{id}", delete(delete_genre))
        // Label routes
        .route("/labels", get(get_labels))
        .route("/labels", post(create_label))
        .route("/labels/{id}", get(get_label_by_id))
        .route("/labels/{id}", put(update_label))
        .route("/labels/{id}", patch(patch_label))
        .route("/labels/{id}", delete(delete_label))
        // Studio routes
        .route("/studios", get(get_studios))
        .route("/studios", post(create_studio))
        .route("/studios/{id}", get(get_studio_by_id))
        .route("/studios/{id}", put(update_studio))
        .route("/studios/{id}", patch(patch_studio))
        .route("/studios/{id}", delete(delete_studio))
        // Series routes
        .route("/series", get(get_series))
        .route("/series", post(create_series))
        .route("/series/{id}", get(get_series_by_id))
        .route("/series/{id}", put(update_series))
        .route("/series/{id}", patch(patch_series))
        .route("/series/{id}", delete(delete_series))
        // Idol routes
        .route("/idols", get(get_idols))
        .route("/idols", post(create_idol))
        .route("/idols/{id}", get(get_idol_by_id))
        .route("/idols/{id}", put(update_idol))
        .route("/idols/{id}", patch(patch_idol))
        .route("/idols/{id}", delete(delete_idol))
        // Record routes
        .route("/records", get(get_records))
        .route("/records", post(create_record))
        .route("/records/{id}", get(get_record_by_id))
        .route("/records/{id}", put(update_record))
        .route("/records/{id}", patch(patch_record))
        .route("/records/{id}", delete(delete_record))
        // Records by entity endpoints
        .route("/director/{id}/records", get(get_records_by_director))
        .route("/studio/{id}/records", get(get_records_by_studio))
        .route("/label/{id}/records", get(get_records_by_label))
        .route("/series/{id}/records", get(get_records_by_series))
        .route("/genre/{id}/records", get(get_records_by_genre))
        .route("/idol/{id}/records", get(get_records_by_idol))
        // Count routes
        .route("/director-records-count", get(get_director_records_count))
        .route("/genre-records-count", get(get_genre_records_count))
        .route("/label-records-count", get(get_label_records_count))
        .route("/studio-records-count", get(get_studio_records_count))
        .route("/series-records-count", get(get_series_records_count))
        .route("/idol-records-count", get(get_idol_records_count))
}
