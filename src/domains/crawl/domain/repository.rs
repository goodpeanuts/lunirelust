use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{DatabaseConnection, DbErr};

use super::model::{
    CrawlCodeResult, CrawlPageResult, CrawlTask, CrawlTaskDetail, EntityAutoCrawlType,
    PageResultStatus, TaskStatus, TaskType,
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

/// A task created for an entity during an atomic entity-auto-crawl claim.
#[derive(Debug, Clone)]
pub struct ClaimedEntity {
    pub entity_id: i64,
    pub entity_name: String,
    pub task_id: i64,
}

/// Aggregated coverage for one entity type (summary endpoint payload data).
#[derive(Debug, Clone)]
pub struct EntityProgressSummaryData {
    pub current_round: i64,
    pub total: u64,
    pub remaining: u64,
    pub failed: u64,
}

/// One entity's progress row (list endpoint item data). `status` is derived
/// from `crawl_task.status` via `last_task_id`; the `last_*` fields are `None`
/// when the entity has no progress row.
#[derive(Debug, Clone)]
pub struct EntityProgressRow {
    pub entity_id: i64,
    pub entity_name: String,
    pub status: String,
    pub last_crawled_round: Option<i32>,
    pub last_crawled_at: Option<DateTime<Utc>>,
    pub last_task_id: Option<i64>,
}

/// Per-entity-type crawl progress for the entity-auto-crawl task type.
///
/// Concurrency guard: the claim operations (`claim_uncrawled`, `claim_failed`)
/// acquire a per-type transaction-scoped advisory lock at the start of their
/// transaction, serializing same-type claims. (The single serial runner already
/// serializes execution, but HTTP request creation can overlap.) This is used
/// instead of `FOR UPDATE OF <entity table>`, which does not protect against
/// the round bump on the joined `crawl_entity_progress` table.
#[async_trait]
pub trait EntityProgressRepository: Send + Sync {
    /// `current_round = COALESCE(MIN(last_crawled_round), 0)` over entities of
    /// the type with non-empty `link` that HAVE a progress row. Empty set -> 0.
    async fn current_round(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
    ) -> Result<i64, DbErr>;

    /// Atomic uncrawled-scope claim: acquire per-type advisory lock, select up
    /// to `count` candidates at `current_round` with no pending task, create one
    /// queued `entity_auto_crawl` task per candidate, and set each entity's
    /// `last_task_id` WITHOUT changing `last_crawled_round` (Model D: claim never
    /// advances the round; a new row is inserted at `last_crawled_round = 0`).
    /// All within one transaction. Returns the created tasks.
    async fn claim_uncrawled(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        count: u32,
        current_round: i64,
        user_id: &str,
        base_url: &str,
    ) -> Result<Vec<ClaimedEntity>, DbErr>;

    /// Atomic failed-scope claim: acquire per-type advisory lock, select up to
    /// `count` entities whose last task is `failed`, create one queued task per
    /// entity, and reassign `last_task_id` WITHOUT changing `last_crawled_round`
    /// (Model D: claim never advances the round). One transaction.
    async fn claim_failed(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        count: u32,
        current_round: i64,
        user_id: &str,
        base_url: &str,
    ) -> Result<Vec<ClaimedEntity>, DbErr>;

    /// Count of uncrawled-scope candidates at `current_round` with non-empty
    /// `link` and no pending task (built on the same `link <> ''` base as
    /// `total`, so it never exceeds `total`).
    async fn count_remaining(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        current_round: i64,
    ) -> Result<u64, DbErr>;

    /// Advance the round on successful completion (Model D: the ONLY writer of
    /// `last_crawled_round`). Sets `last_crawled_round = GREATEST(last_crawled_round,
    /// crawl_round + 1)` (monotonic and idempotent) and updates `last_crawled_at`.
    async fn advance_round_on_complete(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        entity_id: i64,
        crawl_round: i64,
    ) -> Result<(), DbErr>;

    /// Cancel a still-queued entity-auto-crawl task: set `crawl_task.status =
    /// cancelled`. Model D: cancellation never touches `last_crawled_round`, so
    /// no clamp is needed (the entity stays at its round = MIN and is immediately
    /// re-selectable via the `uncrawled` scope).
    async fn cancel_queued_entity_auto_task(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<(), DbErr>;

    /// Update `last_crawled_at` on failure/cancellation. Does not touch
    /// `last_crawled_round` (Model D: round advances only on success).
    async fn touch_on_finalize(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        entity_id: i64,
    ) -> Result<(), DbErr>;

    /// Coverage summary: the current round plus total / remaining / failed counts.
    async fn progress_summary(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
    ) -> Result<EntityProgressSummaryData, DbErr>;

    /// Paginated progress listing with optional derived-status filter (one of
    /// `never` / `in_progress` / `completed` / `failed`). Returns the page and
    /// the total matching count.
    async fn list_progress(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        status: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<EntityProgressRow>, u64), DbErr>;
}
