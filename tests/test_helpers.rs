#![allow(clippy::all)]
#![allow(dead_code)]

use std::sync::Once;

use axum::{
    body::Body,
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        Method, Request, Response, StatusCode,
    },
    Router,
};

use dotenvy::from_filename;
use http_body_util::BodyExt as _;

use lunirelust::{
    app::create_router,
    common::{
        bootstrap::build_app_state,
        config::{setup_database, Config},
        dto::RestApiResponse,
        jwt::{AuthBody, AuthPayload},
    },
};

use sea_orm::DatabaseConnection;
use tower::ServiceExt as _;

static INIT: Once = Once::new();

/// Constants for test client credentials
/// These are used to authenticate the test client
pub const TEST_CLIENT_ID: &str = "apitest01";

pub const TEST_CLIENT_SECRET: &str = "test_password";

pub const TEST_USER_ID: &str = "00000000-0000-0000-0000-000000000001";

/// Helper function to load environment variables from .env.test file
fn load_test_env() {
    INIT.call_once(|| {
        from_filename(".env.test").expect("Failed to load .env.test");

        // uncomment below for test debugging
        // use lunirelust::common::bootstrap::setup_tracing;
        // setup_tracing();
    });
}

/// Helper function to set up the test database state
pub async fn setup_test_db() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    load_test_env();
    let config = Config::from_env()?;
    let pool = setup_database(&config).await?;
    Ok(pool)
}

/// Helper function to create a test router
pub async fn create_test_router() -> Router {
    let pool = setup_test_db().await.expect("Failed to setup test db");
    let config = Config::from_env().expect("Failed to load config");
    let state = build_app_state(&pool, config);
    create_router(state)
}

/// Helper function gets the authentication token
/// for the test client
/// This function is used to authenticate the test client
async fn get_authentication_token() -> String {
    let payload = AuthPayload {
        client_id: TEST_CLIENT_ID.to_owned(),
        client_secret: TEST_CLIENT_SECRET.to_owned(),
    };

    let response = request_with_body(Method::POST, "/auth/login", &payload);

    let (parts, body) = response.await.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Failed to get authentication token"
    );

    let response_body: RestApiResponse<AuthBody> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize response body");
    let auth_body = response_body
        .0
        .data
        .expect("Failed to get auth body from response");
    let token = format!("{} {}", auth_body.token_type, auth_body.access_token);
    token
}

/// Helper function to deserialize the body of a request into a specific type
pub async fn deserialize_json_body<T: serde::de::DeserializeOwned>(
    body: Body,
) -> Result<T, Box<dyn std::error::Error>> {
    let bytes = body
        .collect()
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect response body: {}", e);
            e
        })?
        .to_bytes();

    if bytes.is_empty() {
        return Err(("Empty response body").into());
    }

    // Debugging output
    // Uncomment the following lines to print the response body
    // if let Ok(body) = std::str::from_utf8(&bytes) {
    //     println!("body = {body:?}");
    // }

    let parsed = serde_json::from_slice::<T>(&bytes)?;

    Ok(parsed)
}

/// Helper functions to create a request
pub async fn request(method: Method, uri: &str) -> Response<Body> {
    let request = get_request(method, uri);
    let app = create_test_router().await;

    app.oneshot(request.await).await.unwrap()
}

/// Helper function to create a request with a body
pub async fn request_with_body<T: serde::Serialize>(
    method: Method,
    uri: &str,
    payload: &T,
) -> Response<Body> {
    let json_payload = serde_json::to_string(payload).expect("Failed to serialize payload");
    let request = get_request_with_body(method, uri, &json_payload);
    let app = create_test_router().await;

    app.oneshot(request.await).await.unwrap()
}

/// Helper function to create a request with authentication
pub async fn request_with_auth(method: Method, uri: &str) -> Response<Body> {
    let token = get_authentication_token().await;
    let request = get_request_with_auth(method, uri, &token);
    let app = create_test_router().await;

    app.oneshot(request.await).await.unwrap()
}

/// Helper function to create a request with authentication and a body
pub async fn request_with_auth_and_body<T: serde::Serialize>(
    method: Method,
    uri: &str,
    payload: &T,
) -> Response<Body> {
    let json_payload = serde_json::to_string(payload).expect("Failed to serialize payload");
    let token = get_authentication_token().await;
    let request = get_request_with_auth_and_body(method, uri, &token, &json_payload);
    let app = create_test_router().await;

    app.oneshot(request.await).await.unwrap()
}

/// Helper function to create a request with authentication and multipart data
pub async fn request_with_auth_and_multipart(
    method: Method,
    uri: &str,
    payload: Vec<u8>,
) -> Response<Body> {
    let token = get_authentication_token().await;
    let request = get_request_with_auth_and_multipart(method, uri, &token, payload);
    let app = create_test_router().await;

    app.oneshot(request.await).await.unwrap()
}

/// internal helper functions to create requests
async fn get_request(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri.to_owned())
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .body(axum::body::Body::empty())
        .expect("Failed to create request")
}

/// internal helper function to create a request with a body
async fn get_request_with_body(method: Method, uri: &str, payload: &str) -> Request<Body> {
    let request: Request<Body> = Request::builder()
        .method(method)
        .uri(uri.to_owned())
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .body(axum::body::Body::from(payload.to_owned()))
        .expect("Failed to create request");

    request
}

/// internal helper function to create a request with authorization
async fn get_request_with_auth(method: Method, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri.to_owned())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, token)
        .header(ACCEPT, "application/json")
        .body(axum::body::Body::empty())
        .expect("Failed to create request")
}

async fn get_request_with_auth_and_body(
    method: Method,
    uri: &str,
    token: &str,
    payload: &str,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri.to_owned())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, token)
        .header(ACCEPT, "application/json")
        .body(axum::body::Body::from(payload.to_owned()))
        .expect("Failed to create request")
}

async fn get_request_with_auth_and_multipart(
    method: Method,
    uri: &str,
    token: &str,
    payload: Vec<u8>,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri.to_owned())
        .header(CONTENT_TYPE, "multipart/form-data; boundary=----XYZ")
        .header(AUTHORIZATION, token)
        .header(ACCEPT, "application/json")
        .body(Body::from(payload))
        .expect("Failed to create request")
}
