mod common;
#[path = "test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Once;

use lunirelust::common::bootstrap::setup_tracing;
use lunirelust::common::config::{Config, SearchBackend};
use lunirelust::domains::search::dto::SearchQuery;
use lunirelust::domains::search::{SearchEntityType, SearchService, SearchServiceTrait};
use lunirelust::entities::{search_document_versions, search_sync_events};
use sea_orm::{DatabaseConnection, EntityTrait as _};
use tokio::time::{sleep, Duration, Instant};

use self::common::{spawn_fake_meili_server, FakeEmbedderFailureMode, FakeMeiliServer};

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
    let mut config = Config::from_env()
        .expect("database integration tests require the isolated environment; run `just test`");
    config.search_backend = SearchBackend::MeiliHybrid;
    config.meili_url = server.base_url.clone();
    config.meili_master_key = "test-key".to_owned();
    config.vllm_embedding_url = format!("{}/embedding", server.base_url);
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
    let (service, server) = make_search_service(FakeEmbedderFailureMode::PatchRejected).await;

    service
        .trigger_startup_sync()
        .await
        .expect("start enhanced search");
    wait_until_meili_ready(&service).await;

    assert!(service.is_meili_ready());
    assert!(server.embedder_patch_requests() >= 1);
}

#[tokio::test]
async fn test_startup_sync_reaches_ready_when_embedder_task_fails() {
    let (service, server) = make_search_service(FakeEmbedderFailureMode::TaskFailed).await;

    service
        .trigger_startup_sync()
        .await
        .expect("start enhanced search");
    wait_until_meili_ready(&service).await;

    assert!(service.is_meili_ready());
    assert!(server.embedder_patch_requests() >= 1);
}

#[tokio::test]
async fn test_search_keeps_keyword_only_mode_when_embedder_setup_fails() {
    let (service, server) = make_search_service(FakeEmbedderFailureMode::PatchRejected).await;

    service
        .trigger_startup_sync()
        .await
        .expect("start enhanced search");
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
    assert_eq!(
        server.embedding_requests(),
        0,
        "failed embedder setup must disable vLLM requests even when it is healthy"
    );
    assert_eq!(
        server.vector_search_requests(),
        0,
        "failed embedder setup must disable vector queries"
    );
    assert!(
        response
            .results
            .iter()
            .any(|item| item.entity_type == SearchEntityType::Record && item.id == "ABC-123"),
        "expected Meili keyword search result to be returned"
    );
    assert!(server.keyword_search_requests() >= 1);
}

fn configured_search(server: &FakeMeiliServer, backend: SearchBackend) -> Config {
    let mut config = Config::from_env().expect("use the isolated test environment");
    config.search_backend = backend;
    config.meili_url = server.base_url.clone();
    config.vllm_embedding_url = format!("{}/embedding", server.base_url);
    config
}

fn fixture_query() -> SearchQuery {
    serde_json::from_value(serde_json::json!({"q": "ABC-123", "entity_types": "record"}))
        .expect("search query")
}

#[tokio::test]
async fn keyword_mode_never_contacts_healthy_vllm_or_configures_embedders() {
    let server = spawn_fake_meili_server(FakeEmbedderFailureMode::PatchRejected).await;
    let db = test_helpers::setup_test_db().await.expect("test database");
    let service = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::MeiliKeyword),
        db,
    );
    service
        .trigger_startup_sync()
        .await
        .expect("start keyword search");
    wait_until_meili_ready(&service).await;
    let response = service
        .search(fixture_query(), i32::MAX)
        .await
        .expect("keyword search");
    assert_eq!(response.search_mode, "keyword_only");
    let initial_health_requests = server.health_requests();
    sleep(Duration::from_millis(1100)).await;
    assert_eq!(
        server.health_requests(),
        initial_health_requests,
        "idle worker probed health every second"
    );

    assert_eq!(
        server.embedding_requests(),
        0,
        "keyword mode contacted vLLM"
    );
    assert_eq!(
        server.vector_search_requests(),
        0,
        "keyword mode issued vector search"
    );
    assert_eq!(
        server.embedder_patch_requests(),
        0,
        "keyword mode configured an embedder"
    );
}

#[tokio::test]
async fn explicit_hybrid_mode_uses_healthy_embedding_and_vector_search() {
    let server = spawn_fake_meili_server(FakeEmbedderFailureMode::Healthy).await;
    let db = test_helpers::setup_test_db().await.expect("test database");
    let service = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::MeiliHybrid),
        db,
    );
    service
        .trigger_startup_sync()
        .await
        .expect("start hybrid search");
    wait_until_meili_ready(&service).await;
    let response = service
        .search(fixture_query(), i32::MAX)
        .await
        .expect("hybrid search");
    assert_eq!(response.search_mode, "hybrid");
    assert!(
        server.embedding_requests() > 0,
        "hybrid mode did not use vLLM"
    );
    assert_eq!(
        server.vector_search_requests(),
        1,
        "hybrid mode did not query vectors"
    );
}

#[tokio::test]
#[expect(clippy::too_many_lines)]
async fn reenabling_indexing_refreshes_offline_edits_and_deletes_once() {
    use lunirelust::domains::luna::dto::{CreateDirectorDto, UpdateDirectorDto};
    use lunirelust::domains::luna::{LunaService, LunaServiceTrait};

    let server = spawn_fake_meili_server(FakeEmbedderFailureMode::PatchRejected).await;
    let db = test_helpers::setup_test_db().await.expect("test database");
    let config = configured_search(&server, SearchBackend::MeiliKeyword);
    let catalog = <LunaService as LunaServiceTrait>::create_service(config.clone(), db.clone());
    let name = format!("reenable-{}", uuid::Uuid::new_v4());
    let mut ids = Vec::new();
    for suffix in ["edit", "delete"] {
        let director = catalog
            .director_service()
            .create_director(CreateDirectorDto {
                name: format!("{name}-{suffix}"),
                link: None,
                manual: Some(true),
            })
            .await
            .expect("create director");
        ids.push(director.id);
    }
    let initial = <SearchService as SearchServiceTrait>::create_service(config.clone(), db.clone());
    initial.trigger_startup_sync().await.expect("start index");
    wait_until_meili_ready(&initial).await;
    let edited_doc_id = format!("director__{}", ids[0]);
    let deleted_doc_id = format!("director__{}", ids[1]);
    assert!(
        server.document(&edited_doc_id).await.is_some(),
        "initial index lacks fixture"
    );
    drop(initial);

    // Leave a queued enhanced-mode update behind before indexing is disabled.
    catalog
        .director_service()
        .update_director(
            ids[0],
            UpdateDirectorDto {
                id: ids[0],
                name: Some(format!("{name}-queued")),
                link: None,
                manual: None,
            },
        )
        .await
        .expect("enqueue old update");
    let mut postgres_config = config.clone();
    postgres_config.search_backend = SearchBackend::Postgres;
    let postgres_search =
        <SearchService as SearchServiceTrait>::create_service(postgres_config.clone(), db.clone());
    postgres_search
        .trigger_startup_sync()
        .await
        .expect("persist disabled mode");
    let postgres_catalog =
        <LunaService as LunaServiceTrait>::create_service(postgres_config, db.clone());
    let latest_name = format!("{name}-latest");
    postgres_catalog
        .director_service()
        .update_director(
            ids[0],
            UpdateDirectorDto {
                id: ids[0],
                name: Some(latest_name.clone()),
                link: None,
                manual: None,
            },
        )
        .await
        .expect("edit while indexing disabled");
    postgres_catalog
        .director_service()
        .delete_director(ids[1])
        .await
        .expect("delete while indexing disabled");

    let restarted =
        <SearchService as SearchServiceTrait>::create_service(config.clone(), db.clone());
    restarted
        .trigger_startup_sync()
        .await
        .expect("reenable index");
    wait_until_meili_ready(&restarted).await;
    let refreshed = server
        .document(&edited_doc_id)
        .await
        .expect("refreshed document");
    assert_eq!(
        refreshed["title"], latest_name,
        "reenabling preserved an obsolete title"
    );
    assert!(
        server.document(&deleted_doc_id).await.is_none(),
        "reenabling resurrected a deleted entity"
    );
    drop(restarted);

    let upserts_before_clean_restart = server.document_upserts();
    let clean_restart = <SearchService as SearchServiceTrait>::create_service(config, db);
    clean_restart
        .trigger_startup_sync()
        .await
        .expect("restart healthy index");
    wait_until_meili_ready(&clean_restart).await;
    assert_eq!(
        server.document_upserts(),
        upserts_before_clean_restart,
        "an unchanged enhanced restart must not reindex the entire catalog"
    );
    drop(clean_restart);
    postgres_catalog
        .director_service()
        .delete_director(ids[0])
        .await
        .expect("cleanup director");
    // Cleanup uses PostgreSQL mode, so preserve invalidation for later tests too.
    let mut cleanup_config = Config::from_env().expect("test config");
    cleanup_config.search_backend = SearchBackend::Postgres;
    let cleanup = <SearchService as SearchServiceTrait>::create_service(
        cleanup_config,
        test_helpers::setup_test_db().await.expect("test database"),
    );
    cleanup
        .trigger_startup_sync()
        .await
        .expect("invalidate after cleanup");
}

#[tokio::test]
async fn reenable_rebuild_preserves_an_edit_committed_during_its_snapshot() {
    use lunirelust::domains::luna::dto::CreateDirectorDto;
    use lunirelust::domains::luna::{LunaService, LunaServiceTrait};
    use lunirelust::domains::search::{
        OutboxRepo, OutboxRepository as _, TombstoneRepo, TombstoneRepository as _,
    };
    use sea_orm::{ConnectionTrait as _, DbBackend, Statement, TransactionTrait as _};

    let server = spawn_fake_meili_server(FakeEmbedderFailureMode::PatchRejected).await;
    let db = test_helpers::setup_test_db().await.expect("test database");
    let config = configured_search(&server, SearchBackend::Postgres);
    let postgres_search =
        <SearchService as SearchServiceTrait>::create_service(config.clone(), db.clone());
    postgres_search
        .trigger_startup_sync()
        .await
        .expect("invalidate index");
    let catalog = <LunaService as LunaServiceTrait>::create_service(config, db.clone());
    let name = format!("concurrent-{}", uuid::Uuid::new_v4());
    let director = catalog
        .director_service()
        .create_director(CreateDirectorDto {
            name: name.clone(),
            link: None,
            manual: Some(true),
        })
        .await
        .expect("create director");
    let doc_id = format!("director__{}", director.id);
    let latest_name = format!("{name}-committed-during-sync");

    // This is a normal enhanced write transaction held just before commit.
    // Its event version and sequence ID both predate the index scan, but its
    // database update is invisible to that scan until the transaction commits.
    let txn = db.begin().await.expect("begin concurrent write");
    txn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE director SET name = $1 WHERE id = $2",
        [latest_name.clone().into(), director.id.into()],
    ))
    .await
    .expect("stage catalog update");
    let version = chrono::Utc::now().timestamp_nanos_opt().expect("timestamp");
    OutboxRepo::insert_event(
        &txn,
        "director",
        &director.id.to_string(),
        "upsert",
        version,
        Some(serde_json::json!({"name": latest_name})),
        None,
    )
    .await
    .expect("stage outbox update");
    TombstoneRepo::upsert_version(&txn, "director", &director.id.to_string(), version)
        .await
        .expect("stage entity version");

    server.pause_upsert_for(doc_id.clone()).await;
    let enhanced = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::MeiliKeyword),
        db.clone(),
    );
    enhanced
        .trigger_startup_sync()
        .await
        .expect("start refresh");
    server.wait_for_paused_upsert().await;
    assert!(
        !enhanced.is_meili_ready(),
        "index became ready before the refresh finished"
    );
    txn.commit().await.expect("commit concurrent write");
    server.resume_upsert();
    wait_until_meili_ready(&enhanced).await;
    let document = server.document(&doc_id).await.expect("indexed document");
    assert_eq!(
        document["title"], latest_name,
        "rebuild lost a concurrent committed edit"
    );
    drop(enhanced);
    catalog
        .director_service()
        .delete_director(director.id)
        .await
        .expect("cleanup director");
    let cleanup = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::Postgres),
        db,
    );
    cleanup
        .trigger_startup_sync()
        .await
        .expect("invalidate after cleanup");
}

#[tokio::test]
async fn deep_hybrid_pagination_uses_bounded_sql_without_overflow() {
    let server = spawn_fake_meili_server(FakeEmbedderFailureMode::Healthy).await;
    let db = test_helpers::setup_test_db().await.expect("test database");
    let service = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::MeiliHybrid),
        db,
    );
    service
        .trigger_startup_sync()
        .await
        .expect("start hybrid search");
    wait_until_meili_ready(&service).await;
    let mut query = fixture_query();
    query.offset = Some(i64::MAX);
    query.limit = Some(100);
    let response = service.search(query, 0).await.expect("deep search page");
    assert_eq!(response.search_mode, "sql_fallback");
    assert_eq!(response.offset, i64::MAX);
    assert!(response.results.is_empty());
    assert_eq!(
        server.vector_search_requests(),
        0,
        "deep page issued unbounded vector query"
    );
}

#[tokio::test]
async fn a_failed_rebuild_keeps_the_persisted_generation_dirty_until_retry() {
    use sea_orm::{ConnectionTrait as _, DbBackend, Statement};

    let server = spawn_fake_meili_server(FakeEmbedderFailureMode::PatchRejected).await;
    let db = test_helpers::setup_test_db().await.expect("test database");
    let postgres = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::Postgres),
        db.clone(),
    );
    postgres
        .trigger_startup_sync()
        .await
        .expect("persist invalidation");
    server.set_reject_document_upserts(true);
    let enhanced = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::MeiliKeyword),
        db.clone(),
    );
    enhanced
        .trigger_startup_sync()
        .await
        .expect("start failing rebuild");
    let deadline = Instant::now() + Duration::from_secs(30);
    while server.document_upserts() < 2 && Instant::now() < deadline {
        sleep(Duration::from_millis(20)).await;
    }
    assert!(
        server.document_upserts() >= 2,
        "startup failure did not complete before the retry"
    );
    assert!(
        !enhanced.is_meili_ready(),
        "failed rebuild was exposed to search traffic"
    );
    drop(enhanced);
    let state = db.query_one(Statement::from_string(DbBackend::Postgres,
        "SELECT generation != indexed_generation AS dirty FROM search_index_state WHERE id = TRUE",
    )).await.expect("read invalidation").expect("state row");
    assert!(
        state.try_get::<bool>("", "dirty").expect("dirty flag"),
        "failed rebuild cleared invalidation"
    );

    server.set_reject_document_upserts(false);
    let retry = <SearchService as SearchServiceTrait>::create_service(
        configured_search(&server, SearchBackend::MeiliKeyword),
        db,
    );
    retry.trigger_startup_sync().await.expect("retry rebuild");
    wait_until_meili_ready(&retry).await;
}

#[tokio::test]
async fn an_obsolete_record_fanout_hint_does_not_block_index_readiness() {
    let (ready, pending) = try_missing_record_event(0).await;
    assert!(
        ready,
        "an obsolete best-effort record hint prevented index readiness"
    );
    assert_eq!(pending, 0, "obsolete hint must be acknowledged");
}

#[tokio::test]
async fn a_missing_record_with_a_real_version_still_blocks_readiness() {
    let (ready, pending) = try_missing_record_event(1).await;
    assert!(!ready, "a real missing record was silently acknowledged");
    assert_eq!(pending, 1, "a real missing record must remain retryable");
}

async fn try_missing_record_event(version: i64) -> (bool, i64) {
    use lunirelust::domains::search::{OutboxRepo, OutboxRepository as _};
    use sea_orm::{ConnectionTrait as _, DbBackend, Statement};

    let server = spawn_fake_meili_server(FakeEmbedderFailureMode::PatchRejected).await;
    let db = test_helpers::setup_test_db().await.expect("test database");
    let config = configured_search(&server, SearchBackend::MeiliKeyword);
    let first = <SearchService as SearchServiceTrait>::create_service(config.clone(), db.clone());
    first
        .trigger_startup_sync()
        .await
        .expect("prepare initial index");
    wait_until_meili_ready(&first).await;
    drop(first);

    let deleted_record = format!("obsolete-hint-{}", uuid::Uuid::new_v4());
    OutboxRepo::insert_event(
        &db,
        "record",
        &deleted_record,
        "upsert",
        version,
        None,
        None,
    )
    .await
    .expect("enqueue a hint whose record was deleted");
    let restarted = <SearchService as SearchServiceTrait>::create_service(config, db.clone());
    restarted
        .trigger_startup_sync()
        .await
        .expect("restart index");
    let ready = tokio::time::timeout(Duration::from_secs(3), async {
        while !restarted.is_meili_ready() {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .is_ok();
    drop(restarted);
    let row = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT count(*) AS n FROM search_sync_events WHERE entity_type = 'record' AND entity_id = $1 AND processed_at IS NULL",
        [deleted_record.clone().into()],
    )).await.expect("count hint events").expect("count row");
    let pending: i64 = row.try_get("", "n").expect("count");
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM search_sync_events WHERE entity_type = 'record' AND entity_id = $1",
        [deleted_record.into()],
    ))
    .await
    .expect("cleanup hint");
    (ready, pending)
}
