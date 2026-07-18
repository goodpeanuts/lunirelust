use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use luneth::common::ImageData;
use luneth::crawl::{CrawlError, CrawlInput};
use luneth::record::{RecordPiece, Recorder};
use regex::Regex;
use sea_orm::{DatabaseConnection, DbErr};
use tokio_util::sync::CancellationToken;

use crate::common::config::Config;
use crate::domains::crawl::domain::model::{
    CodeResultStatus, CrawlCodeResult, CrawlPageResult, CrawlTask, CrawlTaskDetail,
    EntityAutoCrawlScope, EntityAutoCrawlTaskInput, EntityAutoCrawlType, PageResultStatus,
    TaskStatus, TaskType,
};
use crate::domains::crawl::domain::repository::{
    ClaimedEntity, CrawlTaskRepository, EntityProgressRepository, EntityProgressRow,
    EntityProgressSummaryData,
};
use crate::domains::crawl::domain::service::CrawlerTrait;
use crate::domains::crawl::dto::task_dto::SseEvent;
use crate::domains::crawl::infra::crawler::CrawlTaskManager;
use crate::domains::luna::infra::{impl_service::file::FileService, RecordRepo};
use crate::domains::user::InteractionRepo;

use super::CrawlService;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedCodeResult {
    task_id: i64,
    code: String,
    status: String,
    record_id: Option<String>,
    images_downloaded: i32,
    error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedFinalize {
    task_id: i64,
    status: String,
    error_message: Option<String>,
}

struct RecordingCrawlTaskRepo {
    created_code_results: Arc<Mutex<Vec<RecordedCodeResult>>>,
    finalized: Arc<Mutex<Vec<RecordedFinalize>>>,
}

impl RecordingCrawlTaskRepo {
    #[expect(clippy::type_complexity)]
    fn new() -> (
        Self,
        Arc<Mutex<Vec<RecordedCodeResult>>>,
        Arc<Mutex<Vec<RecordedFinalize>>>,
    ) {
        let created_code_results = Arc::new(Mutex::new(Vec::new()));
        let finalized = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                created_code_results: Arc::clone(&created_code_results),
                finalized: Arc::clone(&finalized),
            },
            created_code_results,
            finalized,
        )
    }
}

#[async_trait]
impl CrawlTaskRepository for RecordingCrawlTaskRepo {
    async fn create_task(
        &self,
        _db: &DatabaseConnection,
        _task_type: &TaskType,
        _status: &TaskStatus,
        _user_id: &str,
        _mark_liked: bool,
        _mark_viewed: bool,
        _input_payload: Option<&str>,
        _max_pages: Option<i32>,
        _total_codes: i32,
    ) -> Result<CrawlTask, DbErr> {
        unreachable!("unused in test")
    }

    async fn update_task_status(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
        _status: &TaskStatus,
        _error_message: Option<&str>,
    ) -> Result<(), DbErr> {
        unreachable!("unused in test")
    }

    async fn update_task_started(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
    ) -> Result<(), DbErr> {
        Ok(())
    }

    async fn update_task_counts(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
        _success_count: i32,
        _fail_count: i32,
        _skip_count: i32,
        _total_codes: i32,
    ) -> Result<(), DbErr> {
        Ok(())
    }

    async fn complete_task(
        &self,
        _db: &DatabaseConnection,
        task_id: i64,
        status: &TaskStatus,
        _success_count: i32,
        _fail_count: i32,
        _skip_count: i32,
        _total_codes: i32,
        error_message: Option<&str>,
    ) -> Result<(), DbErr> {
        self.finalized
            .lock()
            .expect("finalized lock poisoned")
            .push(RecordedFinalize {
                task_id,
                status: status.as_str().to_owned(),
                error_message: error_message.map(ToOwned::to_owned),
            });
        Ok(())
    }

    async fn get_task_by_id(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
    ) -> Result<Option<CrawlTask>, DbErr> {
        unreachable!("unused in test")
    }

    async fn list_tasks(
        &self,
        _db: &DatabaseConnection,
        _user_id: &str,
        _status_filter: Option<&TaskStatus>,
        _task_type_filter: Option<&TaskType>,
        _page: u64,
        _page_size: u64,
    ) -> Result<(Vec<CrawlTask>, u64), DbErr> {
        unreachable!("unused in test")
    }

    async fn create_code_result(
        &self,
        _db: &DatabaseConnection,
        task_id: i64,
        code: &str,
        status: &str,
        record_id: Option<&str>,
        images_downloaded: i32,
        error_message: Option<&str>,
    ) -> Result<CrawlCodeResult, DbErr> {
        self.created_code_results
            .lock()
            .expect("recorded code results lock poisoned")
            .push(RecordedCodeResult {
                task_id,
                code: code.to_owned(),
                status: status.to_owned(),
                record_id: record_id.map(ToOwned::to_owned),
                images_downloaded,
                error_message: error_message.map(ToOwned::to_owned),
            });

        Ok(CrawlCodeResult {
            id: 1,
            task_id,
            code: code.to_owned(),
            status: CodeResultStatus::from_str(status).expect("valid code result status"),
            record_id: record_id.map(ToOwned::to_owned),
            images_downloaded,
            error_message: error_message.map(ToOwned::to_owned),
            created_at: Utc::now(),
        })
    }

    async fn list_code_results(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
    ) -> Result<Vec<CrawlCodeResult>, DbErr> {
        unreachable!("unused in test")
    }

    async fn create_page_result(
        &self,
        _db: &DatabaseConnection,
        task_id: i64,
        page_number: i32,
        status: &PageResultStatus,
        records_found: i32,
        error_message: Option<&str>,
    ) -> Result<CrawlPageResult, DbErr> {
        Ok(CrawlPageResult {
            id: 1,
            task_id,
            page_number,
            status: status.clone(),
            records_found,
            records_crawled: 0,
            error_message: error_message.map(ToOwned::to_owned),
            created_at: Utc::now(),
        })
    }

    async fn update_page_result(
        &self,
        _db: &DatabaseConnection,
        _id: i64,
        _status: &PageResultStatus,
        _records_crawled: i32,
        _error_message: Option<&str>,
    ) -> Result<(), DbErr> {
        Ok(())
    }

    async fn list_page_results(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
    ) -> Result<Vec<CrawlPageResult>, DbErr> {
        Ok(vec![])
    }

    async fn find_tasks_by_status(
        &self,
        _db: &DatabaseConnection,
        _statuses: &[TaskStatus],
    ) -> Result<Vec<CrawlTask>, DbErr> {
        unreachable!("unused in test")
    }

    async fn fail_processing_page_results(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
        _error_message: &str,
    ) -> Result<(), DbErr> {
        Ok(())
    }

    async fn get_task_detail(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
    ) -> Result<Option<CrawlTaskDetail>, DbErr> {
        unreachable!("unused in test")
    }

    async fn count_code_results_by_status(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
    ) -> Result<(i32, i32, i32), DbErr> {
        unreachable!("unused in test")
    }
}

fn test_config() -> Config {
    Config {
        database_url: "postgres://example.invalid/test".to_owned(),
        database_max_connections: 1,
        database_min_connections: 0,
        service_host: "127.0.0.1".to_owned(),
        service_port: "3000".to_owned(),
        assets_public_path: "/tmp".to_owned(),
        assets_public_url: "http://localhost/public".to_owned(),
        assets_private_path: "/tmp".to_owned(),
        assets_private_url: "http://localhost/private".to_owned(),
        asset_allowed_extensions_pattern: Regex::new(r".*").expect("regex"),
        asset_allowed_extensions: vec!["jpg".to_owned()],
        asset_max_size: 1024,
        cors_origins: vec![],
        meili_url: "http://localhost:7700".to_owned(),
        meili_master_key: "test".to_owned(),
        vllm_embedding_url: "http://localhost:8000".to_owned(),
        vllm_embedding_model: "test".to_owned(),
        vllm_embedding_timeout_secs: 5,
    }
}

/// No-op `EntityProgressRepository` for tests that do not exercise the
/// entity-auto-crawl flow (keeps the existing batch/auto/update/idol tests
/// compiling after the new dependency was added to `CrawlService`).
struct NoopEntityProgressRepo;

#[async_trait]
impl EntityProgressRepository for NoopEntityProgressRepo {
    async fn current_round(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
    ) -> Result<i64, DbErr> {
        Ok(0)
    }
    async fn claim_uncrawled(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
        _count: u32,
        _current_round: i64,
        _user_id: &str,
        _base_url: &str,
    ) -> Result<Vec<ClaimedEntity>, DbErr> {
        Ok(vec![])
    }
    async fn claim_failed(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
        _count: u32,
        _current_round: i64,
        _user_id: &str,
        _base_url: &str,
    ) -> Result<Vec<ClaimedEntity>, DbErr> {
        Ok(vec![])
    }
    async fn count_remaining(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
        _current_round: i64,
    ) -> Result<u64, DbErr> {
        Ok(0)
    }
    async fn advance_round_on_complete(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
        _entity_id: i64,
        _crawl_round: i64,
    ) -> Result<(), DbErr> {
        Ok(())
    }
    async fn cancel_queued_entity_auto_task(
        &self,
        _db: &DatabaseConnection,
        _task_id: i64,
    ) -> Result<(), DbErr> {
        Ok(())
    }
    async fn touch_on_finalize(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
        _entity_id: i64,
    ) -> Result<(), DbErr> {
        Ok(())
    }
    async fn progress_summary(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
    ) -> Result<EntityProgressSummaryData, DbErr> {
        Ok(EntityProgressSummaryData {
            current_round: 0,
            total: 0,
            remaining: 0,
            failed: 0,
        })
    }
    async fn list_progress(
        &self,
        _db: &DatabaseConnection,
        _entity_type: EntityAutoCrawlType,
        _status: Option<&str>,
        _page: u64,
        _page_size: u64,
    ) -> Result<(Vec<EntityProgressRow>, u64), DbErr> {
        Ok((vec![], 0))
    }
}

/// Mock `RecordRepository` for executor tests. `find_by_id` returns `Err` so
/// each record piece resolves to a per-code `failed` result without touching a
/// real database (the dummy `DatabaseConnection` is disconnected and would
/// panic on any `SeaORM` query). Other methods are unreachable in the executor
/// path.
struct MockRecordRepo;

#[async_trait]
impl crate::domains::luna::RecordRepository for MockRecordRepo {
    async fn find_by_id(
        &self,
        _db: &DatabaseConnection,
        _id: String,
    ) -> Result<Option<crate::domains::luna::Record>, DbErr> {
        Err(DbErr::Custom(
            "mock record repo: find_by_id disabled".to_owned(),
        ))
    }
    async fn find_all(
        &self,
        _db: &DatabaseConnection,
    ) -> Result<Vec<crate::domains::luna::Record>, DbErr> {
        unreachable!()
    }
    async fn find_list(
        &self,
        _db: &DatabaseConnection,
        _search_dto: crate::domains::luna::dto::SearchRecordDto,
    ) -> Result<Vec<crate::domains::luna::Record>, DbErr> {
        unreachable!()
    }
    async fn find_list_paginated(
        &self,
        _db: &DatabaseConnection,
        _search_dto: crate::domains::luna::dto::SearchRecordDto,
        _pagination: crate::domains::luna::dto::PaginationQuery,
        _user_filter: Option<crate::domains::luna::dto::UserFilter>,
    ) -> Result<crate::domains::luna::dto::PaginatedResponse<crate::domains::luna::Record>, DbErr>
    {
        unreachable!()
    }
    async fn create(
        &self,
        _txn: &sea_orm::DatabaseTransaction,
        _record: crate::domains::luna::dto::CreateRecordDto,
    ) -> Result<(String, crate::domains::luna::CreatedNestedEntities), DbErr> {
        unreachable!()
    }
    async fn update(
        &self,
        _txn: &sea_orm::DatabaseTransaction,
        _id: String,
        _record: crate::domains::luna::dto::UpdateRecordDto,
    ) -> Result<Option<crate::domains::luna::Record>, DbErr> {
        unreachable!()
    }
    async fn delete(
        &self,
        _txn: &sea_orm::DatabaseTransaction,
        _id: String,
    ) -> Result<bool, DbErr> {
        unreachable!()
    }
    async fn update_record_links(
        &self,
        _txn: &sea_orm::DatabaseTransaction,
        _record_id: String,
        _new_links: Vec<crate::domains::luna::dto::CreateLinkDto>,
    ) -> Result<i32, DbErr> {
        unreachable!()
    }
    async fn find_all_slim(
        &self,
        _db: &DatabaseConnection,
        _user_filter: Option<crate::domains::luna::dto::UserFilter>,
    ) -> Result<Vec<crate::domains::luna::Record>, DbErr> {
        unreachable!()
    }
    async fn find_all_ids(
        &self,
        _db: &DatabaseConnection,
        _user_filter: Option<crate::domains::luna::dto::UserFilter>,
    ) -> Result<Vec<String>, DbErr> {
        unreachable!()
    }
    async fn find_ids_paginated(
        &self,
        _db: &DatabaseConnection,
        _pagination: crate::domains::luna::dto::PaginationQuery,
        _user_filter: Option<crate::domains::luna::dto::UserFilter>,
    ) -> Result<crate::domains::luna::dto::PaginatedResponse<String>, DbErr> {
        unreachable!()
    }
    async fn find_all_slim_paginated(
        &self,
        _db: &DatabaseConnection,
        _pagination: crate::domains::luna::dto::PaginationQuery,
        _user_filter: Option<crate::domains::luna::dto::UserFilter>,
    ) -> Result<crate::domains::luna::dto::PaginatedResponse<crate::domains::luna::Record>, DbErr>
    {
        unreachable!()
    }
    async fn find_by_genre_id(
        &self,
        _db: &DatabaseConnection,
        _genre_id: i64,
    ) -> Result<Vec<crate::domains::luna::Record>, DbErr> {
        unreachable!()
    }
    async fn find_by_genre_id_paginated(
        &self,
        _db: &DatabaseConnection,
        _genre_id: i64,
        _pagination: crate::domains::luna::dto::PaginationQuery,
        _user_filter: Option<crate::domains::luna::dto::UserFilter>,
    ) -> Result<crate::domains::luna::dto::PaginatedResponse<crate::domains::luna::Record>, DbErr>
    {
        unreachable!()
    }
    async fn find_by_idol_id(
        &self,
        _db: &DatabaseConnection,
        _idol_id: i64,
    ) -> Result<Vec<crate::domains::luna::Record>, DbErr> {
        unreachable!()
    }
    async fn find_by_idol_id_paginated(
        &self,
        _db: &DatabaseConnection,
        _idol_id: i64,
        _pagination: crate::domains::luna::dto::PaginationQuery,
        _user_filter: Option<crate::domains::luna::dto::UserFilter>,
    ) -> Result<crate::domains::luna::dto::PaginatedResponse<crate::domains::luna::Record>, DbErr>
    {
        unreachable!()
    }
}

#[expect(clippy::type_complexity)]
fn test_service(
    repo: RecordingCrawlTaskRepo,
    record_repo: Arc<dyn crate::domains::luna::RecordRepository + Send + Sync>,
) -> (
    CrawlService,
    tokio::sync::broadcast::Receiver<SseEvent>,
    Arc<Mutex<Vec<RecordedCodeResult>>>,
    Arc<Mutex<Vec<RecordedFinalize>>>,
) {
    let (tx, rx) = tokio::sync::broadcast::channel(32);
    let manager = Arc::new(tokio::sync::Mutex::new(CrawlTaskManager::new(tx)));
    let db = DatabaseConnection::default();
    let config = test_config();
    let created_code_results = Arc::clone(&repo.created_code_results);
    let finalized = Arc::clone(&repo.finalized);
    let service = CrawlService::new(
        db,
        config.clone(),
        Arc::new(repo),
        Arc::new(NoopEntityProgressRepo),
        Arc::new(InteractionRepo),
        record_repo,
        Arc::new(FileService::new(config)),
        manager,
    );
    (service, rx, created_code_results, finalized)
}

fn drain_events(rx: &mut tokio::sync::broadcast::Receiver<SseEvent>) -> Vec<SseEvent> {
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    events
}

#[tokio::test]
async fn persist_code_result_and_emit_progress_records_and_broadcasts_failed_code() {
    let (repo, _, _) = RecordingCrawlTaskRepo::new();
    let (service, mut rx, created_code_results, _) = test_service(repo, Arc::new(RecordRepo));

    service
        .persist_code_result_and_emit_progress(
            42,
            "user-1",
            "ABP-123",
            "failed",
            None,
            0,
            Some("crawler failed"),
        )
        .await;

    let created_code_results = created_code_results
        .lock()
        .expect("recorded code results lock poisoned")
        .clone();
    assert_eq!(
        created_code_results,
        vec![RecordedCodeResult {
            task_id: 42,
            code: "ABP-123".to_owned(),
            status: "failed".to_owned(),
            record_id: None,
            images_downloaded: 0,
            error_message: Some("crawler failed".to_owned()),
        }]
    );

    let events = drain_events(&mut rx);
    assert!(
        events.iter().any(|event| matches!(
            event,
            SseEvent::CodeProgress {
                task_id: 42,
                user_id,
                code,
                status,
                record_id: None,
                images_downloaded: 0,
            } if user_id == "user-1" && code == "ABP-123" && status == "failed"
        )),
        "missing failed code-progress event: {events:?}"
    );
}

// ---- Entity auto crawl executor tests (mock CrawlerTrait) ----

/// Scripted per-page outcome for the mock crawler.
enum PageOutcome {
    Records(Vec<RecordPiece>),
    Empty,
    NotFound,
    OtherError(String),
}

/// Mock crawler that replays a scripted sequence of page outcomes. Each
/// `crawl_page` call pops the next outcome; once exhausted it returns 404
/// (treats the listing as ended).
struct ScriptedCrawler {
    pages: Mutex<VecDeque<PageOutcome>>,
}

impl ScriptedCrawler {
    fn new(pages: Vec<PageOutcome>) -> Self {
        Self {
            pages: Mutex::new(pages.into()),
        }
    }
}

#[async_trait(?Send)]
impl CrawlerTrait for ScriptedCrawler {
    async fn set_base_url(&self, _base_url: String) -> Result<(), CrawlError> {
        Ok(())
    }

    async fn crawl_page(&self, url: &str) -> Result<Vec<RecordPiece>, CrawlError> {
        let outcome = self.pages.lock().expect("pages lock poisoned").pop_front();
        match outcome {
            Some(PageOutcome::Records(p)) => Ok(p),
            Some(PageOutcome::Empty) => Ok(vec![]),
            Some(PageOutcome::OtherError(m)) => {
                Err(CrawlError::RecordPieceExtractionFailed { err: m })
            }
            // 404 and "script exhausted" both end the listing.
            Some(PageOutcome::NotFound) | None => Err(CrawlError::PageNotFound {
                url: url.to_owned(),
            }),
        }
    }

    async fn crawl_recorder_with_imgs(
        &self,
        _input: CrawlInput,
    ) -> Result<(Recorder, Arc<Vec<ImageData>>), CrawlError> {
        // process_single_code calls record_repo.find_by_id first (dummy DB ->
        // Err), so each record resolves to Failed before this is reached.
        unreachable!("crawl_recorder_with_imgs not reached in executor tests")
    }

    async fn crawl_idol_image(&self, _link: &str) -> Result<ImageData, CrawlError> {
        unreachable!("crawl_idol_image not reached")
    }
}

fn piece(code: &str) -> RecordPiece {
    RecordPiece {
        title: format!("t-{code}"),
        url: format!("https://example.com/{code}"),
        code: code.to_owned(),
        date: "2024-01-01".to_owned(),
        display_image_url: String::new(),
    }
}

fn entity_input() -> EntityAutoCrawlTaskInput {
    EntityAutoCrawlTaskInput {
        entity_type: EntityAutoCrawlType::Idol,
        entity_id: 1,
        entity_name: "test".to_owned(),
        link: "https://example.com/star/test".to_owned(),
        base_url: "https://example.com/".to_owned(),
        crawl_round: 0,
        scope: EntityAutoCrawlScope::Uncrawled,
    }
}

/// Run the executor with a scripted crawler and return the recorded
/// `complete_task` call(s).
async fn run_entity_auto(
    crawler: &ScriptedCrawler,
    cancel: CancellationToken,
) -> Vec<RecordedFinalize> {
    let (repo, _, finalized) = RecordingCrawlTaskRepo::new();
    let (service, _rx, _, _) = test_service(repo, Arc::new(MockRecordRepo));
    service
        .execute_entity_auto_crawl_task(7, entity_input(), "u".to_owned(), crawler, cancel)
        .await;
    let recorded = finalized.lock().expect("finalized lock poisoned").clone();
    recorded
}

fn assert_single_finalize(recorded: &[RecordedFinalize]) -> &RecordedFinalize {
    assert_eq!(recorded.len(), 1, "expected exactly one complete_task call");
    &recorded[0]
}

#[tokio::test]
async fn entity_auto_clean_run_then_404_is_completed() {
    // Two content pages then a 404 -> Completed, no error message.
    let crawler = ScriptedCrawler::new(vec![
        PageOutcome::Records(vec![piece("AAA-1")]),
        PageOutcome::Records(vec![piece("AAA-2")]),
        PageOutcome::NotFound,
    ]);
    let recorded = run_entity_auto(&crawler, CancellationToken::new()).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.task_id, 7);
    assert_eq!(fin.status, "completed");
    assert!(fin.error_message.is_none());
}

#[tokio::test]
async fn entity_auto_midpage_non404_error_then_recover_is_failed() {
    // content, non-404 error, content, 404 -> Failed (had an error page), no
    // breaker message.
    let crawler = ScriptedCrawler::new(vec![
        PageOutcome::Records(vec![piece("AAA-1")]),
        PageOutcome::OtherError("boom".to_owned()),
        PageOutcome::Records(vec![piece("AAA-2")]),
        PageOutcome::NotFound,
    ]);
    let recorded = run_entity_auto(&crawler, CancellationToken::new()).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.status, "failed");
    assert!(fin.error_message.is_none());
}

#[tokio::test]
async fn entity_auto_broken_link_first_page_404_is_failed() {
    // 404 on page 1, no content -> Failed (broken link).
    let crawler = ScriptedCrawler::new(vec![PageOutcome::NotFound]);
    let recorded = run_entity_auto(&crawler, CancellationToken::new()).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.status, "failed");
    assert!(fin.error_message.is_none());
}

#[tokio::test]
async fn entity_auto_all_errors_until_404_is_failed() {
    // Three non-404 errors then a 404, no content -> Failed.
    let crawler = ScriptedCrawler::new(vec![
        PageOutcome::OtherError("e1".to_owned()),
        PageOutcome::OtherError("e2".to_owned()),
        PageOutcome::OtherError("e3".to_owned()),
        PageOutcome::NotFound,
    ]);
    let recorded = run_entity_auto(&crawler, CancellationToken::new()).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.status, "failed");
    assert!(fin.error_message.is_none());
}

#[tokio::test]
async fn entity_auto_empty_page_does_not_stop() {
    // Empty page (success, 0 records) is not a stop condition: iteration
    // continues past it to a content page, then ends at 404 -> Completed.
    let crawler = ScriptedCrawler::new(vec![
        PageOutcome::Empty,
        PageOutcome::Records(vec![piece("AAA-1")]),
        PageOutcome::NotFound,
    ]);
    let recorded = run_entity_auto(&crawler, CancellationToken::new()).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.status, "completed");
}

#[tokio::test]
async fn entity_auto_cancel_is_cancelled() {
    // Pre-cancelled token: the executor breaks before page 1 -> Cancelled.
    let crawler = ScriptedCrawler::new(vec![PageOutcome::Records(vec![piece("AAA-1")])]);
    let token = CancellationToken::new();
    token.cancel();
    let recorded = run_entity_auto(&crawler, token).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.status, "cancelled");
}

#[tokio::test]
async fn entity_auto_circuit_breaker_trips() {
    // ENTITY_AUTO_CRAWL_MAX_CONSECUTIVE_ERRORS (10) consecutive non-404 errors
    // with no intervening content trip the breaker -> Failed + "Circuit breaker:".
    let mut pages: Vec<PageOutcome> = (0..10)
        .map(|i| PageOutcome::OtherError(format!("e{i}")))
        .collect();
    // A trailing 404 that must NEVER be reached (the breaker stops first).
    pages.push(PageOutcome::NotFound);
    let crawler = ScriptedCrawler::new(pages);
    let recorded = run_entity_auto(&crawler, CancellationToken::new()).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.status, "failed");
    let msg = fin
        .error_message
        .as_ref()
        .expect("breaker must set an error message");
    assert!(
        msg.starts_with("Circuit breaker:"),
        "expected breaker message, got: {msg}"
    );
}

#[tokio::test]
async fn entity_auto_intermittent_errors_do_not_trip_breaker() {
    // Errors interspersed with content reset the counter, so the breaker never
    // trips; iteration ends at the first 404. Status is Failed (error pages
    // present) but with NO breaker message.
    let crawler = ScriptedCrawler::new(vec![
        PageOutcome::OtherError("e1".to_owned()),
        PageOutcome::Records(vec![piece("AAA-1")]),
        PageOutcome::OtherError("e3".to_owned()),
        PageOutcome::OtherError("e4".to_owned()),
        PageOutcome::Records(vec![piece("AAA-2")]),
        PageOutcome::NotFound,
    ]);
    let recorded = run_entity_auto(&crawler, CancellationToken::new()).await;
    let fin = assert_single_finalize(&recorded);
    assert_eq!(fin.status, "failed");
    assert!(
        fin.error_message.is_none(),
        "intermittent errors must not trip the breaker: {:?}",
        fin.error_message
    );
}
