#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Method, StatusCode};
use axum::response::Response;

use lunirelust::common::dto::RestApiResponse;

mod test_helpers;

use test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

#[tokio::test]
async fn test_batch_rejects_empty_codes() {
    let body = serde_json::json!({
        "codes": [],
        "mark_liked": false,
        "mark_viewed": false
    });
    let resp: Response<Body> =
        request_with_auth_and_body(Method::POST, "/crawl/batch", &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_auto_rejects_zero_max_pages() {
    let body = serde_json::json!({
        "start_url": "https://example.com",
        "max_pages": 0
    });
    let resp: Response<Body> = request_with_auth_and_body(Method::POST, "/crawl/auto", &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_rejects_no_filters() {
    let body = serde_json::json!({
        "liked_only": false
    });
    let resp: Response<Body> =
        request_with_auth_and_body(Method::POST, "/crawl/update", &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_tasks_returns_ok() {
    let resp: Response<Body> = request_with_auth(Method::GET, "/crawl/tasks").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (_, body) = resp.into_parts();
    let result: RestApiResponse<serde_json::Value> = deserialize_json_body(body).await.unwrap();
    assert!(result.0.data.is_some());
}

#[tokio::test]
async fn test_get_task_detail_not_found() {
    let resp: Response<Body> = request_with_auth(Method::GET, "/crawl/tasks/999999").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_cancel_nonexistent_task() {
    let resp: Response<Body> = request_with_auth(Method::POST, "/crawl/tasks/999999/cancel").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_batch_creates_task() {
    let body = serde_json::json!({
        "codes": ["ABC-123", "DEF-456"],
        "mark_liked": true,
        "mark_viewed": false
    });
    let resp: Response<Body> =
        request_with_auth_and_body(Method::POST, "/crawl/batch", &body).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let (_, body) = resp.into_parts();
    let result: RestApiResponse<serde_json::Value> = deserialize_json_body(body).await.unwrap();
    let data = result.0.data.unwrap();
    assert!(data["task_id"].as_i64().unwrap() > 0);
    let status = data["status"].as_str().unwrap();
    assert!(status == "running" || status == "queued");
}

#[tokio::test]
async fn test_update_with_future_date_returns_error() {
    let body = serde_json::json!({
        "liked_only": false,
        "created_after": "2099-12-31"
    });
    let resp: Response<Body> =
        request_with_auth_and_body(Method::POST, "/crawl/update", &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
