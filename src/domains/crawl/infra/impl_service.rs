mod impl_auto;
mod impl_batch;
mod impl_entity_auto_crawl;
mod impl_helpers;
mod impl_idol;
mod impl_update;

#[cfg(test)]
mod impl_tests;

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde_json;

use crate::common::config::Config;
use crate::common::error::AppError;
use crate::domains::crawl::domain::model::{
    AutoTaskInput, BatchTaskInput, CrawlTask, CrawlTaskDetail, CrawlTaskInput,
    EntityAutoCrawlScope, EntityAutoCrawlTaskInput, EntityAutoCrawlType, IdolTaskInput, TaskStatus,
    TaskType, UpdateFilters, UpdateTaskInput,
};
use crate::domains::crawl::domain::repository::{CrawlTaskRepository, EntityProgressRepository};
use crate::domains::crawl::domain::service::CrawlServiceTrait;
use crate::domains::crawl::dto::task_dto::{
    EntityAutoCrawlTaskItem, EntityAutoCrawlTaskResponse, EntityProgressItem,
    EntityProgressListResponse, EntityProgressSummary, SseEvent, TaskSummary,
};
use crate::domains::crawl::infra::crawler::{CancelAction, CrawlTaskManager, CrawlerStatus};
use crate::domains::luna::{FileServiceTrait as LunaFileServiceTrait, RecordRepository};
use crate::domains::user::InteractionRepository;

pub struct CrawlService {
    pub(super) db: DatabaseConnection,
    pub(super) config: Config,
    pub(super) repo: Arc<dyn CrawlTaskRepository + Send + Sync>,
    pub(super) entity_repo: Arc<dyn EntityProgressRepository + Send + Sync>,
    pub(super) interaction_repo: Arc<dyn InteractionRepository + Send + Sync>,
    pub(super) record_repo: Arc<dyn RecordRepository + Send + Sync>,
    #[expect(
        dead_code,
        reason = "file_service used for image serving in crawl workflows"
    )]
    pub(super) file_service: Arc<dyn LunaFileServiceTrait + Send + Sync>,
    pub(super) task_manager: Arc<tokio::sync::Mutex<CrawlTaskManager>>,
}

impl CrawlService {
    fn resolve_base_url(base_url: Option<String>) -> Result<String, AppError> {
        match base_url {
            Some(url) => {
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(AppError::ValidationError(
                        "base_url must start with http:// or https://".to_owned(),
                    ));
                }
                let mut url = url;
                if !url.ends_with('/') {
                    url.push('/');
                }
                Ok(url)
            }
            None => Ok(luneth::crawl::CrawlConfig::default().base_url),
        }
    }

    /// Parse an `EntityAutoCrawlTaskInput` from a task's persisted payload, if it
    /// is an entity-auto-crawl task. Used by `cancel_task` (to read the scope for
    /// the round clamp) and `reconcile_startup` (to locate the entity).
    fn parse_entity_auto_input(payload: &Option<String>) -> Option<EntityAutoCrawlTaskInput> {
        let p = payload.as_deref()?;
        let input: CrawlTaskInput = serde_json::from_str(p).ok()?;
        match input {
            CrawlTaskInput::EntityAutoCrawl(ei) => Some(ei),
            _ => None,
        }
    }

    #[expect(clippy::too_many_arguments)]
    pub fn new(
        db: DatabaseConnection,
        config: Config,
        repo: Arc<dyn CrawlTaskRepository + Send + Sync>,
        entity_repo: Arc<dyn EntityProgressRepository + Send + Sync>,
        interaction_repo: Arc<dyn InteractionRepository + Send + Sync>,
        record_repo: Arc<dyn RecordRepository + Send + Sync>,
        file_service: Arc<dyn LunaFileServiceTrait + Send + Sync>,
        task_manager: Arc<tokio::sync::Mutex<CrawlTaskManager>>,
    ) -> Self {
        Self {
            db,
            config,
            repo,
            entity_repo,
            interaction_repo,
            record_repo,
            file_service,
            task_manager,
        }
    }
}

#[async_trait]
impl CrawlServiceTrait for CrawlService {
    fn create_service(
        db: DatabaseConnection,
        config: Config,
        crawl_repo: Arc<dyn CrawlTaskRepository + Send + Sync>,
        entity_repo: Arc<dyn EntityProgressRepository + Send + Sync>,
        interaction_repo: Arc<dyn InteractionRepository + Send + Sync>,
        record_repo: Arc<dyn RecordRepository + Send + Sync>,
        file_service: Arc<dyn LunaFileServiceTrait + Send + Sync>,
        task_manager: Arc<tokio::sync::Mutex<CrawlTaskManager>>,
    ) -> Arc<dyn CrawlServiceTrait> {
        Arc::new(Self {
            db,
            config,
            repo: crawl_repo,
            entity_repo,
            interaction_repo,
            record_repo,
            file_service,
            task_manager,
        })
    }

    async fn start_batch(
        &self,
        user_id: &str,
        codes: Vec<String>,
        mark_liked: bool,
        mark_viewed: bool,
        base_url: Option<String>,
    ) -> Result<(i64, TaskStatus), AppError> {
        let canonical_codes: Vec<String> = codes.into_iter().map(|c| c.to_uppercase()).collect();
        let total = canonical_codes.len() as i32;
        let base_url = Self::resolve_base_url(base_url)?;

        let input = CrawlTaskInput::Batch(BatchTaskInput {
            codes: canonical_codes.clone(),
            base_url,
            mark_liked,
            mark_viewed,
        });
        let payload = serde_json::to_string(&input).map_err(|e| {
            AppError::InternalErrorWithMessage(format!("Failed to serialize input: {e}"))
        })?;

        let task = self
            .repo
            .create_task(
                &self.db,
                &TaskType::Batch,
                &TaskStatus::Queued,
                user_id,
                mark_liked,
                mark_viewed,
                Some(&payload),
                None,
                total,
            )
            .await
            .map_err(AppError::DatabaseError)?;

        let task_id = task.id;
        let mut mgr = self.task_manager.lock().await;
        let started = mgr.enqueue(task_id);
        let status = if started {
            TaskStatus::Running
        } else {
            TaskStatus::Queued
        };

        Ok((task_id, status))
    }

    async fn start_auto(
        &self,
        user_id: &str,
        start_url: String,
        max_pages: u32,
        mark_liked: bool,
        mark_viewed: bool,
        append_page_path: bool,
        base_url: Option<String>,
    ) -> Result<(i64, TaskStatus), AppError> {
        let base_url = Self::resolve_base_url(base_url)?;

        let input = CrawlTaskInput::Auto(AutoTaskInput {
            start_url,
            max_pages,
            base_url,
            mark_liked,
            mark_viewed,
            append_page_path,
        });
        let payload = serde_json::to_string(&input).map_err(|e| {
            AppError::InternalErrorWithMessage(format!("Failed to serialize input: {e}"))
        })?;

        let task = self
            .repo
            .create_task(
                &self.db,
                &TaskType::Auto,
                &TaskStatus::Queued,
                user_id,
                mark_liked,
                mark_viewed,
                Some(&payload),
                Some(max_pages as i32),
                0,
            )
            .await
            .map_err(AppError::DatabaseError)?;

        let task_id = task.id;
        let mut mgr = self.task_manager.lock().await;
        let started = mgr.enqueue(task_id);
        let status = if started {
            TaskStatus::Running
        } else {
            TaskStatus::Queued
        };

        Ok((task_id, status))
    }

    async fn start_update(
        &self,
        user_id: &str,
        codes: Option<Vec<String>>,
        liked_only: bool,
        created_after: Option<String>,
        base_url: Option<String>,
        update_images: bool,
    ) -> Result<(i64, TaskStatus), AppError> {
        let target_ids = if let Some(ref c) = codes {
            c.iter().map(|s| s.to_uppercase()).collect()
        } else {
            self.resolve_update_targets(user_id, liked_only, created_after.as_deref())
                .await?
        };

        if target_ids.is_empty() {
            return Err(AppError::ValidationError(
                "No records match the specified filters".to_owned(),
            ));
        }

        let base_url = Self::resolve_base_url(base_url)?;
        let filters = UpdateFilters {
            liked_only,
            created_after: created_after.clone(),
        };
        let input = CrawlTaskInput::Update(UpdateTaskInput {
            filters,
            target_ids: target_ids.clone(),
            base_url,
            update_images,
        });
        let payload = serde_json::to_string(&input).map_err(|e| {
            AppError::InternalErrorWithMessage(format!("Failed to serialize input: {e}"))
        })?;

        let total = target_ids.len() as i32;
        let task = self
            .repo
            .create_task(
                &self.db,
                &TaskType::Update,
                &TaskStatus::Queued,
                user_id,
                false,
                false,
                Some(&payload),
                None,
                total,
            )
            .await
            .map_err(AppError::DatabaseError)?;

        let task_id = task.id;
        let mut mgr = self.task_manager.lock().await;
        let started = mgr.enqueue(task_id);
        let status = if started {
            TaskStatus::Running
        } else {
            TaskStatus::Queued
        };

        Ok((task_id, status))
    }

    async fn start_idol(
        &self,
        user_id: &str,
        base_url: Option<String>,
    ) -> Result<(i64, TaskStatus), AppError> {
        let base_url = Self::resolve_base_url(base_url)?;

        let idols = self.resolve_idols_without_images().await?;

        if idols.is_empty() {
            return Err(AppError::ValidationError(
                "All idols already have images".to_owned(),
            ));
        }

        let total = idols.len() as i32;
        let input = CrawlTaskInput::Idol(IdolTaskInput { base_url, idols });
        let payload = serde_json::to_string(&input).map_err(|e| {
            AppError::InternalErrorWithMessage(format!("Failed to serialize input: {e}"))
        })?;

        let task = self
            .repo
            .create_task(
                &self.db,
                &TaskType::Idol,
                &TaskStatus::Queued,
                user_id,
                false,
                false,
                Some(&payload),
                None,
                total,
            )
            .await
            .map_err(AppError::DatabaseError)?;

        let task_id = task.id;
        let mut mgr = self.task_manager.lock().await;
        let started = mgr.enqueue(task_id);
        let status = if started {
            TaskStatus::Running
        } else {
            TaskStatus::Queued
        };

        Ok((task_id, status))
    }

    async fn cancel_task(&self, user_id: &str, task_id: i64) -> Result<(), AppError> {
        let task = self
            .repo
            .get_task_by_id(&self.db, task_id)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Task not found".to_owned()))?;

        if task.user_id != user_id {
            return Err(AppError::NotFound("Task not found".to_owned()));
        }

        if task.status.is_terminal() {
            return Err(AppError::Conflict(
                "Cannot cancel a completed/failed/cancelled task".to_owned(),
            ));
        }

        let mut mgr = self.task_manager.lock().await;
        match mgr.cancel_task(task_id) {
            // Running: only signal cancellation via the token. The executor owns
            // the terminal transition (finalizes Cancelled + scope-aware round
            // clamp for EntityAutoCrawl), avoiding a race with normal finalize.
            CancelAction::Running => {}
            CancelAction::RemovedFromQueue => {
                drop(mgr);
                if Self::parse_entity_auto_input(&task.input_payload).is_some() {
                    // EntityAutoCrawl (Model D): cancellation only marks the task
                    // cancelled; it never touches the progress row's round. The
                    // entity stays at its round (= MIN) and is immediately
                    // re-selectable via the uncrawled scope.
                    self.entity_repo
                        .cancel_queued_entity_auto_task(&self.db, task_id)
                        .await
                        .map_err(AppError::DatabaseError)?;
                } else {
                    // Non-entity-auto task, or unreadable payload: mark cancelled
                    // without any progress-row side effect.
                    self.repo
                        .complete_task(
                            &self.db,
                            task_id,
                            &TaskStatus::Cancelled,
                            task.success_count,
                            task.fail_count,
                            task.skip_count,
                            task.total_codes,
                            None,
                        )
                        .await
                        .map_err(AppError::DatabaseError)?;
                }

                let pages_crawled = self.count_successful_pages(task_id).await;
                let mgr = self.task_manager.lock().await;
                mgr.emit_event(SseEvent::TaskCancelled {
                    task_id,
                    user_id: user_id.to_owned(),
                    summary: TaskSummary {
                        total: task.total_codes,
                        success: task.success_count,
                        failed: task.fail_count,
                        skipped: task.skip_count,
                        pages_crawled,
                    },
                });
                // Do NOT call complete_current() here -- the running task is still active.
            }
            CancelAction::NotFound => {
                return Err(AppError::NotFound("Task not found in queue".to_owned()));
            }
        }

        Ok(())
    }

    async fn list_tasks(
        &self,
        user_id: &str,
        status_filter: Option<TaskStatus>,
        task_type_filter: Option<TaskType>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<CrawlTask>, u64), AppError> {
        self.repo
            .list_tasks(
                &self.db,
                user_id,
                status_filter.as_ref(),
                task_type_filter.as_ref(),
                page,
                page_size,
            )
            .await
            .map_err(AppError::DatabaseError)
    }

    async fn get_task_detail(
        &self,
        user_id: &str,
        task_id: i64,
    ) -> Result<Option<CrawlTaskDetail>, AppError> {
        let detail = self
            .repo
            .get_task_detail(&self.db, task_id)
            .await
            .map_err(AppError::DatabaseError)?;

        match detail {
            Some(d) if d.task.user_id == user_id => Ok(Some(d)),
            _ => Ok(None),
        }
    }

    async fn start_entity_auto_crawl(
        &self,
        user_id: &str,
        entity_type: EntityAutoCrawlType,
        count: u32,
        scope: EntityAutoCrawlScope,
        base_url: Option<String>,
    ) -> Result<EntityAutoCrawlTaskResponse, AppError> {
        let base_url = Self::resolve_base_url(base_url)?;
        let current_round = self
            .entity_repo
            .current_round(&self.db, entity_type)
            .await
            .map_err(AppError::DatabaseError)?;

        // Atomic claim: select + round-effect + task creation in one transaction,
        // guarded by a per-type advisory lock.
        let claimed = match scope {
            EntityAutoCrawlScope::Uncrawled => {
                self.entity_repo
                    .claim_uncrawled(
                        &self.db,
                        entity_type,
                        count,
                        current_round,
                        user_id,
                        &base_url,
                    )
                    .await
            }
            EntityAutoCrawlScope::Failed => {
                self.entity_repo
                    .claim_failed(
                        &self.db,
                        entity_type,
                        count,
                        current_round,
                        user_id,
                        &base_url,
                    )
                    .await
            }
        }
        .map_err(AppError::DatabaseError)?;

        let mut tasks = Vec::with_capacity(claimed.len());
        {
            let mut mgr = self.task_manager.lock().await;
            for c in &claimed {
                mgr.enqueue(c.task_id);
                tasks.push(EntityAutoCrawlTaskItem {
                    entity_id: c.entity_id,
                    entity_name: c.entity_name.clone(),
                    task_id: c.task_id,
                });
            }
        }

        let remaining = self
            .entity_repo
            .count_remaining(&self.db, entity_type, current_round)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(EntityAutoCrawlTaskResponse {
            tasks,
            picked: claimed.len() as u32,
            remaining,
        })
    }

    async fn list_entity_progress(
        &self,
        entity_type: EntityAutoCrawlType,
        status: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<EntityProgressListResponse, AppError> {
        let status_filter = match status.as_deref() {
            Some(s) if matches!(s, "never" | "in_progress" | "completed" | "failed") => Some(s),
            Some(_) => {
                return Err(AppError::ValidationError(
                    "status must be one of: never, in_progress, completed, failed".to_owned(),
                ));
            }
            None => None,
        };
        let (rows, total) = self
            .entity_repo
            .list_progress(&self.db, entity_type, status_filter, page, page_size)
            .await
            .map_err(AppError::DatabaseError)?;
        let items = rows
            .into_iter()
            .map(|r| EntityProgressItem {
                entity_id: r.entity_id,
                entity_name: r.entity_name,
                status: r.status,
                last_crawled_round: r.last_crawled_round,
                last_crawled_at: r.last_crawled_at,
                last_task_id: r.last_task_id,
            })
            .collect();
        Ok(EntityProgressListResponse { items, total })
    }

    async fn get_entity_progress_summary(
        &self,
        entity_type: EntityAutoCrawlType,
    ) -> Result<EntityProgressSummary, AppError> {
        let s = self
            .entity_repo
            .progress_summary(&self.db, entity_type)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(EntityProgressSummary {
            entity_type: entity_type.as_str().to_owned(),
            current_round: s.current_round,
            total: s.total,
            remaining: s.remaining,
            failed: s.failed,
        })
    }

    fn task_manager(&self) -> Arc<tokio::sync::Mutex<CrawlTaskManager>> {
        self.task_manager.clone()
    }

    async fn initialize_crawler(&self) -> Result<(), AppError> {
        let mgr = self.task_manager.lock().await;

        if mgr.initialized() {
            return Ok(());
        }

        if !mgr.is_idle() {
            return Err(AppError::Conflict(
                "Crawler is busy executing a task".to_owned(),
            ));
        }

        mgr.send_initialize()
            .map_err(|e| AppError::InternalErrorWithMessage(format!("Failed to send init: {e}")))?;

        let mut rx = mgr
            .init_tx_clone()
            .ok_or_else(|| AppError::InternalErrorWithMessage("Init watcher not set".to_owned()))?
            .subscribe();

        drop(mgr);

        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            // Wait for the value to change from the initial false.
            while rx.changed().await.is_ok() {
                if *rx.borrow() {
                    return Ok(());
                }
            }
            Err(AppError::InternalErrorWithMessage(
                "Init watcher channel closed".to_owned(),
            ))
        })
        .await
        .map_err(|_elapsed| {
            AppError::InternalErrorWithMessage(
                "Crawler initialization timed out after 60s".to_owned(),
            )
        })?
    }

    async fn crawler_status(&self) -> CrawlerStatus {
        let mgr = self.task_manager.lock().await;
        CrawlerStatus {
            initialized: mgr.initialized(),
            idle: mgr.is_idle(),
        }
    }

    #[expect(clippy::let_underscore_must_use, clippy::let_underscore_untyped)]
    async fn reconcile_startup(&self) {
        let tasks = match self
            .repo
            .find_tasks_by_status(&self.db, &[TaskStatus::Running, TaskStatus::Queued])
            .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to query stale crawl tasks: {e}");
                return;
            }
        };

        let mut queued_ids = Vec::new();

        for task in tasks {
            match task.status {
                TaskStatus::Running => {
                    if let Err(e) = self
                        .repo
                        .fail_processing_page_results(
                            &self.db,
                            task.id,
                            "Page processing interrupted: server restarted",
                        )
                        .await
                    {
                        tracing::warn!("Failed to clean up page results for task {}: {e}", task.id);
                    }

                    // Recover actual counts from code_result rows instead of
                    // using potentially stale counters on the task itself.
                    let (success, failed, skipped) = self
                        .repo
                        .count_code_results_by_status(&self.db, task.id)
                        .await
                        .unwrap_or((task.success_count, task.fail_count, task.skip_count));

                    if let Err(e) = self
                        .repo
                        .complete_task(
                            &self.db,
                            task.id,
                            &TaskStatus::Failed,
                            success,
                            failed,
                            skipped,
                            task.total_codes,
                            Some("Server restarted during execution"),
                        )
                        .await
                    {
                        tracing::warn!("Failed to mark running task {} as failed: {e}", task.id);
                    }

                    // EntityAutoCrawl: stamp last_crawled_at. The round is left
                    // unchanged (the bump stays), so the entity is selectable via
                    // the failed scope for a retry.
                    if let Some(ei) = Self::parse_entity_auto_input(&task.input_payload) {
                        if let Err(e) = self
                            .entity_repo
                            .touch_on_finalize(&self.db, ei.entity_type, ei.entity_id)
                            .await
                        {
                            tracing::warn!(
                                "Failed to touch entity progress for task {}: {e}",
                                task.id
                            );
                        }
                    }
                }
                TaskStatus::Queued => {
                    let payload_valid = task
                        .input_payload
                        .as_deref()
                        .is_some_and(|p| serde_json::from_str::<CrawlTaskInput>(p).is_ok());
                    if payload_valid {
                        queued_ids.push(task.id);
                    } else {
                        tracing::warn!(
                            "Queued task {} has invalid or missing payload, marking failed",
                            task.id
                        );
                        let _ = self
                            .repo
                            .complete_task(
                                &self.db,
                                task.id,
                                &TaskStatus::Failed,
                                0,
                                0,
                                0,
                                task.total_codes,
                                Some("Queued-task recovery failed: invalid persisted payload"),
                            )
                            .await;
                    }
                }
                _ => unreachable!(),
            }
        }

        if !queued_ids.is_empty() {
            let count = queued_ids.len();
            let mut mgr = self.task_manager.lock().await;
            mgr.reconcile_startup(queued_ids);
            tracing::info!("Re-enqueued {count} queued crawl task(s)");
        }
    }
}
