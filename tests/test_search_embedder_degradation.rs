mod common;
#[path = "test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Once;

use lunirelust::common::bootstrap::setup_tracing;
use lunirelust::common::config::Config;
use lunirelust::domains::search::dto::SearchQuery;
use lunirelust::domains::search::{SearchEntityType, SearchService, SearchServiceTrait};
use lunirelust::entities::{search_document_versions, search_sync_events};
use once_cell::sync::Lazy;
use sea_orm::{DatabaseConnection, EntityTrait as _};
use tokio::time::{sleep, Duration, Instant};

use self::common::{spawn_fake_meili_server, FakeEmbedderFailureMode, FakeMeiliServer};

// These tests mutate the shared .env.test database and rely on one fake Meili
// instance at a time, so run them serially to keep startup timing deterministic.
static TEST_MUTEX: Lazy<tokio::sync::Mutex<()>> = Lazy::new(|| tokio::sync::Mutex::new(()));
static TRACING_INIT: Once = Once::new();

fn ensure_tracing() {
    TRACING_INIT.call_once(setup_tracing);
}

async fn reset_search_state(db: &DatabaseConnection) {
    // Give each startup-sync test a clean search pipeline baseline so "ready"
    // reflects the embedder-degradation behavior rather than leftover outbox work.
    search_sync_events::Entity::delete_many()
        .exec(db)
        .await
        .expect("Failed to clear search_sync_events");
    search_document_versions::Entity::delete_many()
        .exec(db)
        .await
        .expect("Failed to clear search_document_versions");
}

async fn make_search_service(
    failure_mode: FakeEmbedderFailureMode,
) -> (Arc<dyn SearchServiceTrait>, FakeMeiliServer) {
    ensure_tracing();
    let server = spawn_fake_meili_server(failure_mode).await;
    let db = test_helpers::setup_test_db()
        .await
        .expect("Failed to setup test db");
    reset_search_state(&db).await;
    let mut config = Config::from_env().expect("Failed to load config");
    config.meili_url = server.base_url.clone();
    config.meili_master_key = "test-key".to_owned();
    config.vllm_embedding_url = "http://127.0.0.1:9".to_owned();
    config.vllm_embedding_model = "BAAI/bge-m3".to_owned();

    let service = <SearchService as SearchServiceTrait>::create_service(config, db);
    (service, server)
}

async fn wait_until_meili_ready(service: &Arc<dyn SearchServiceTrait>) {
    let deadline = Instant::now() + Duration::from_secs(60);

    while Instant::now() < deadline {
        if service.is_meili_ready() {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }

    panic!("MeiliSearch did not become ready within 60 seconds");
}

#[tokio::test]
async fn test_startup_sync_reaches_ready_when_embedder_patch_is_rejected() {
    let _guard = TEST_MUTEX.lock().await;
    let (service, server) = make_search_service(FakeEmbedderFailureMode::PatchRejected).await;

    service.trigger_startup_sync();
    wait_until_meili_ready(&service).await;

    assert!(service.is_meili_ready());
    assert!(server.embedder_patch_requests() >= 1);
}

#[tokio::test]
async fn test_startup_sync_reaches_ready_when_embedder_task_fails() {
    let _guard = TEST_MUTEX.lock().await;
    let (service, server) = make_search_service(FakeEmbedderFailureMode::TaskFailed).await;

    service.trigger_startup_sync();
    wait_until_meili_ready(&service).await;

    assert!(service.is_meili_ready());
    assert!(server.embedder_patch_requests() >= 1);
}

#[tokio::test]
async fn test_search_keeps_keyword_only_mode_when_embedder_setup_fails() {
    let _guard = TEST_MUTEX.lock().await;
    let (service, server) = make_search_service(FakeEmbedderFailureMode::PatchRejected).await;

    service.trigger_startup_sync();
    wait_until_meili_ready(&service).await;

    // Once startup has degraded to keyword-only mode, a real search request
    // should still hit Meili and return keyword_only instead of sql_fallback.
    let response = service
        .search(
            SearchQuery {
                q: "ABC-123".to_owned(),
                entity_types: Some("record".to_owned()),
                director: None,
                studio: None,
                label: None,
                genre: None,
                date_from: None,
                date_to: None,
                limit: Some(20),
                offset: Some(0),
            },
            i32::MAX,
        )
        .await
        .expect("search should continue to use Meili keyword mode");

    assert_eq!(response.search_mode, "keyword_only");
    assert!(
        response
            .results
            .iter()
            .any(|item| item.entity_type == SearchEntityType::Record && item.id == "ABC-123"),
        "expected Meili keyword search result to be returned"
    );
    assert!(server.keyword_search_requests() >= 1);
}
