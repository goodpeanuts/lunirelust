//! Regressions for streaming and bounded extraction through the real HTTP router.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as _;
use lunirelust::app::create_router;
use lunirelust::common::bootstrap::build_app_state;
use lunirelust::common::config::{setup_test_database, Config};
use lunirelust::common::jwt::{Claims, KEYS};
use sea_orm::{ConnectionTrait as _, DbBackend, Statement};
use tower::ServiceExt as _;

#[tokio::test]
async fn debug_logging_preserves_live_sse_and_request_limits() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::sink)
        .try_init()
        .expect("install DEBUG subscriber for the streaming regression");
    let mut config = Config::from_env().expect("use .env.test");
    config.asset_max_size = 128;
    let db = setup_test_database(&config).await.expect("test database");
    let user = "00000000-0000-0000-0000-000000000001";
    let row = db.query_one(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO crawl_task (task_type, status, user_id, mark_liked, mark_viewed, total_codes, success_count, fail_count, skip_count, created_at) VALUES ('batch', 'running', $1, false, false, 1, 0, 0, 0, now()) RETURNING id",
        [user.into()],
    )).await.expect("insert task").expect("task ID");
    let id: i64 = row.try_get("", "id").expect("task ID");
    let app = create_router(build_app_state(&db, config));
    let claims = Claims {
        sub: user.to_owned(),
        ..Claims::default()
    };
    let token = jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &KEYS.encoding)
        .expect("test token");
    let request = Request::builder()
        .uri(format!("/crawl/tasks/{id}/stream"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("SSE request");
    let streaming = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let response = app.clone().oneshot(request).await.expect("SSE response");
        assert_eq!(response.status(), StatusCode::OK);
        let mut body = response.into_body();
        let frame = body
            .frame()
            .await
            .expect("initial frame")
            .expect("valid frame");
        let data = frame.into_data().expect("data frame");
        assert!(String::from_utf8_lossy(&data).contains("task:stats"));
    })
    .await;
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM crawl_task WHERE id = $1",
        [id.into()],
    ))
    .await
    .expect("remove task");
    assert!(
        streaming.is_ok(),
        "SSE must send its initial event without waiting for task completion, even at DEBUG"
    );
    let request = Request::builder()
        .method("POST")
        .uri("/crawl/batch")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"codes": ["A".repeat(512)]}).to_string(),
        ))
        .expect("large JSON request");
    let response = app.oneshot(request).await.expect("bounded response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
