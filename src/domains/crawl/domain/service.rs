use std::sync::Arc;

use async_trait::async_trait;
use luneth::common::ImageData;
use luneth::crawl::{CrawlError, CrawlInput};
use luneth::record::{RecordPiece, Recorder};
use sea_orm::DatabaseConnection;

use crate::common::config::Config;
use crate::common::error::AppError;
use crate::domains::crawl::domain::model::{CrawlTask, CrawlTaskDetail, TaskStatus, TaskType};
use crate::domains::crawl::infra::crawler::CrawlTaskManager;
use crate::domains::luna::{FileServiceTrait as LunaFileServiceTrait, RecordRepository};
use crate::domains::user::InteractionRepository;

use super::repository::CrawlTaskRepository;

/// Abstraction for crawl operations. Not required to be Send/Sync
/// because it runs exclusively on the dedicated crawl runner thread.
#[async_trait(?Send)]
pub trait CrawlerTrait {
    async fn crawl_page(&self, url: &str) -> Result<Vec<RecordPiece>, CrawlError>;
    async fn crawl_recorder_with_imgs(
        &self,
        input: CrawlInput,
    ) -> Result<(Recorder, Arc<Vec<ImageData>>), CrawlError>;
}

#[async_trait]
pub trait CrawlServiceTrait: Send + Sync {
    fn create_service(
        db: DatabaseConnection,
        config: Config,
        crawl_repo: Arc<dyn CrawlTaskRepository + Send + Sync>,
        interaction_repo: Arc<dyn InteractionRepository + Send + Sync>,
        record_repo: Arc<dyn RecordRepository + Send + Sync>,
        file_service: Arc<dyn LunaFileServiceTrait + Send + Sync>,
        task_manager: Arc<tokio::sync::Mutex<CrawlTaskManager>>,
    ) -> Arc<dyn CrawlServiceTrait>
    where
        Self: Sized;

    async fn start_batch(
        &self,
        user_id: &str,
        codes: Vec<String>,
        mark_liked: bool,
        mark_viewed: bool,
    ) -> Result<(i64, TaskStatus), AppError>;

    async fn start_auto(
        &self,
        user_id: &str,
        start_url: String,
        max_pages: u32,
        mark_liked: bool,
        mark_viewed: bool,
    ) -> Result<(i64, TaskStatus), AppError>;

    async fn start_update(
        &self,
        user_id: &str,
        liked_only: bool,
        created_after: Option<String>,
    ) -> Result<(i64, TaskStatus), AppError>;

    async fn cancel_task(&self, user_id: &str, task_id: i64) -> Result<(), AppError>;
    async fn list_tasks(
        &self,
        user_id: &str,
        status_filter: Option<TaskStatus>,
        task_type_filter: Option<TaskType>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<CrawlTask>, u64), AppError>;

    async fn get_task_detail(
        &self,
        user_id: &str,
        task_id: i64,
    ) -> Result<Option<CrawlTaskDetail>, AppError>;

    fn task_manager(&self) -> Arc<tokio::sync::Mutex<CrawlTaskManager>>;

    async fn reconcile_startup(&self);
}
