use axum::{
    error_handling::HandleErrorLayer,
    extract::DefaultBodyLimit,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method, StatusCode,
    },
    middleware,
    response::IntoResponse,
    Router,
};

use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

use crate::{
    common::{
        app_state::AppState,
        error::{handle_error, AppError},
        jwt,
    },
    domains::{
        auth::user_auth_routes, crawl::crawl_routes, device::device_routes, file::file_routes,
        luna::luna_routes, search::search_routes, user::user_routes,
    },
};

#[cfg(feature = "swagger")]
use utoipa::OpenApi as _;

#[cfg(feature = "swagger")]
use crate::domains::{
    auth::UserAuthApiDoc, crawl::CrawlApiDoc, device::DeviceApiDoc, file::FileApiDoc,
    luna::LunaApiDoc, search::SearchApiDoc, user::UserApiDoc,
};

#[cfg(feature = "swagger")]
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "swagger")]
fn create_swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/docs")
        .url(
            "/api-docs/user_auth/openapi.json",
            UserAuthApiDoc::openapi(),
        )
        .url("/api-docs/user/openapi.json", UserApiDoc::openapi())
        .url("/api-docs/device/openapi.json", DeviceApiDoc::openapi())
        .url("/api-docs/file/openapi.json", FileApiDoc::openapi())
        .url("/api-docs/luna/openapi.json", LunaApiDoc::openapi())
        .url("/api-docs/search/openapi.json", SearchApiDoc::openapi())
        .url("/api-docs/crawl/openapi.json", CrawlApiDoc::openapi())
}

pub fn create_router(state: AppState) -> Router {
    // Build a CORS layer from configured origins
    let cors = if state.config.cors_origins.is_empty() {
        tracing::warn!("CORS_ORIGINS not configured — all cross-origin requests will be rejected");
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    } else {
        let origins: Vec<HeaderValue> = state
            .config
            .cors_origins
            .iter()
            .filter_map(|o| o.parse::<HeaderValue>().ok())
            .collect();
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_origin(origins)
            .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    };

    // Create a common middleware stack for error handling, timeouts, and CORS.
    let middleware_stack = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(handle_error))
        .timeout(Duration::from_secs(60))
        .layer(cors);

    let auth_router = Router::new().nest("/auth", user_auth_routes());

    // Protected API routes
    let protected_routes = Router::new()
        .nest("/user", user_routes())
        .nest("/device", device_routes())
        .nest("/file", file_routes())
        .nest("/cards", luna_routes().merge(search_routes()))
        .nest("/crawl", crawl_routes())
        // by default, Multipart limits to 2MB; override with `asset_max_size`
        // See https://docs.rs/axum/latest/axum/extract/struct.Multipart.html
        .layer(DefaultBodyLimit::max(state.config.asset_max_size))
        // enforce JWT authentication
        .route_layer(middleware::from_fn(jwt::jwt_auth));

    // setup assets routes
    let public_assets_routes = Router::new().nest_service(
        state.config.assets_public_url.as_str(),
        ServeDir::new(state.config.assets_public_path.clone()),
    );

    let private_assets_routes = Router::new()
        .nest_service(
            state.config.assets_private_url.as_str(),
            ServeDir::new(state.config.assets_private_path.clone()),
        )
        // enforce JWT authentication
        .route_layer(middleware::from_fn(jwt::jwt_auth));
    // Note: No heavy middleware for static assets to improve performance

    // Create the main router
    // and merge all the routes
    // and add the middleware stack
    // and add the state
    let router = Router::new()
        .route("/health", axum::routing::get(health_check))
        .merge(auth_router)
        .merge(protected_routes)
        .merge(public_assets_routes)
        .merge(private_assets_routes);

    // Conditionally add Swagger UI only if the feature is enabled
    #[cfg(feature = "swagger")]
    let router = router.merge(create_swagger_ui());

    router
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &axum::http::Request<_>| {
                    tracing::info_span!(
                        "request",
                        method = %req.method(),
                        path = %req.uri().path(),
                    )
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            "request completed: status = {status}, latency = {latency:?}",
                            status = response.status(),
                            latency = latency
                        );
                    },
                ),
        )
        .fallback(fallback)
        .layer(middleware_stack)
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK\n"
}

/// Fallback handler for unmatched routes
/// This function returns a 404 Not Found response with a message.
pub async fn fallback() -> Result<impl IntoResponse, AppError> {
    Ok((StatusCode::NOT_FOUND, "Not Found"))
}
