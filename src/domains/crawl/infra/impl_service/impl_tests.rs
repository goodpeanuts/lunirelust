use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use sea_orm::{DatabaseConnection, DbErr};

use crate::common::config::Config;
use crate::domains::crawl::domain::model::{
    CodeResultStatus, CrawlCodeResult, CrawlPageResult, CrawlTask, CrawlTaskDetail,
    PageResultStatus, TaskStatus, TaskType,
};
use crate::domains::crawl::domain::repository::CrawlTaskRepository;
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

struct RecordingCrawlTaskRepo {
    created_code_results: Arc<Mutex<Vec<RecordedCodeResult>>>,
}

impl RecordingCrawlTaskRepo {
    fn new() -> (Self, Arc<Mutex<Vec<RecordedCodeResult>>>) {
        let created_code_results = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                created_code_results: Arc::clone(&created_code_results),
            },
            created_code_results,
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
        _task_id: i64,
        _status: &TaskStatus,
        _success_count: i32,
        _fail_count: i32,
        _skip_count: i32,
        _total_codes: i32,
        _error_message: Option<&str>,
    ) -> Result<(), DbErr> {
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
        _task_id: i64,
        _page_number: i32,
        _status: &PageResultStatus,
        _records_found: i32,
        _error_message: Option<&str>,
    ) -> Result<CrawlPageResult, DbErr> {
        unreachable!("unused in test")
    }

    async fn update_page_result(
        &self,
        _db: &DatabaseConnection,
        _id: i64,
        _status: &PageResultStatus,
        _records_crawled: i32,
        _error_message: Option<&str>,
    ) -> Result<(), DbErr> {
        unreachable!("unused in test")
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

fn test_service(
    repo: RecordingCrawlTaskRepo,
) -> (
    CrawlService,
    tokio::sync::broadcast::Receiver<SseEvent>,
    Arc<Mutex<Vec<RecordedCodeResult>>>,
) {
    let (tx, rx) = tokio::sync::broadcast::channel(32);
    let manager = Arc::new(tokio::sync::Mutex::new(CrawlTaskManager::new(tx)));
    let db = DatabaseConnection::default();
    let config = test_config();
    let created_code_results = Arc::clone(&repo.created_code_results);
    let service = CrawlService::new(
        db,
        config.clone(),
        Arc::new(repo),
        Arc::new(InteractionRepo),
        Arc::new(RecordRepo),
        Arc::new(FileService::new(config)),
        manager,
    );
    (service, rx, created_code_results)
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
    let (repo, _) = RecordingCrawlTaskRepo::new();
    let (service, mut rx, created_code_results) = test_service(repo);

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
