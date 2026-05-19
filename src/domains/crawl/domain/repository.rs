use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr};

use super::model::{
    CrawlCodeResult, CrawlPageResult, CrawlTask, CrawlTaskDetail, PageResultStatus, TaskStatus,
    TaskType,
};

#[async_trait]
#[expect(clippy::too_many_arguments)]
pub trait CrawlTaskRepository: Send + Sync {
    async fn create_task(
        &self,
        db: &DatabaseConnection,
        task_type: &TaskType,
        status: &TaskStatus,
        user_id: &str,
        mark_liked: bool,
        mark_viewed: bool,
        input_payload: Option<&str>,
        max_pages: Option<i32>,
        total_codes: i32,
    ) -> Result<CrawlTask, DbErr>;

    async fn update_task_status(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        status: &TaskStatus,
        error_message: Option<&str>,
    ) -> Result<(), DbErr>;

    async fn update_task_started(&self, db: &DatabaseConnection, task_id: i64)
        -> Result<(), DbErr>;

    async fn update_task_counts(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        success_count: i32,
        fail_count: i32,
        skip_count: i32,
        total_codes: i32,
    ) -> Result<(), DbErr>;

    async fn complete_task(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        status: &TaskStatus,
        success_count: i32,
        fail_count: i32,
        skip_count: i32,
        total_codes: i32,
        error_message: Option<&str>,
    ) -> Result<(), DbErr>;

    async fn get_task_by_id(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Option<CrawlTask>, DbErr>;

    async fn list_tasks(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        status_filter: Option<&TaskStatus>,
        task_type_filter: Option<&TaskType>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<CrawlTask>, u64), DbErr>;

    async fn create_code_result(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        code: &str,
        status: &str,
        record_id: Option<&str>,
        images_downloaded: i32,
        error_message: Option<&str>,
    ) -> Result<CrawlCodeResult, DbErr>;

    async fn list_code_results(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Vec<CrawlCodeResult>, DbErr>;

    async fn create_page_result(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        page_number: i32,
        status: &PageResultStatus,
        records_found: i32,
        error_message: Option<&str>,
    ) -> Result<CrawlPageResult, DbErr>;

    async fn update_page_result(
        &self,
        db: &DatabaseConnection,
        id: i64,
        status: &PageResultStatus,
        records_crawled: i32,
        error_message: Option<&str>,
    ) -> Result<(), DbErr>;

    async fn list_page_results(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Vec<CrawlPageResult>, DbErr>;

    async fn find_tasks_by_status(
        &self,
        db: &DatabaseConnection,
        statuses: &[TaskStatus],
    ) -> Result<Vec<CrawlTask>, DbErr>;

    async fn fail_processing_page_results(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        error_message: &str,
    ) -> Result<(), DbErr>;

    async fn get_task_detail(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Option<CrawlTaskDetail>, DbErr>;

    async fn count_code_results_by_status(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<(i32, i32, i32), DbErr>;
}
