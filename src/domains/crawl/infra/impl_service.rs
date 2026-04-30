mod impl_auto;
mod impl_batch;
mod impl_helpers;
mod impl_update;

#[cfg(test)]
mod impl_tests;

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde_json;

use crate::common::config::Config;
use crate::common::error::AppError;
use crate::domains::crawl::domain::model::CrawlTaskInput;
use crate::domains::crawl::domain::model::{
    AutoTaskInput, BatchTaskInput, CrawlTask, CrawlTaskDetail, TaskStatus, TaskType, UpdateFilters,
    UpdateTaskInput,
};
use crate::domains::crawl::domain::repository::CrawlTaskRepository;
use crate::domains::crawl::domain::service::CrawlServiceTrait;
use crate::domains::crawl::dto::task_dto::{SseEvent, TaskSummary};
use crate::domains::crawl::infra::crawler::{CancelAction, CrawlTaskManager};
use crate::domains::luna::{FileServiceTrait as LunaFileServiceTrait, RecordRepository};
use crate::domains::user::InteractionRepository;

pub struct CrawlService {
    pub(super) db: DatabaseConnection,
    pub(super) config: Config,
    pub(super) repo: Arc<dyn CrawlTaskRepository + Send + Sync>,
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
    pub fn new(
        db: DatabaseConnection,
        config: Config,
        repo: Arc<dyn CrawlTaskRepository + Send + Sync>,
        interaction_repo: Arc<dyn InteractionRepository + Send + Sync>,
        record_repo: Arc<dyn RecordRepository + Send + Sync>,
        file_service: Arc<dyn LunaFileServiceTrait + Send + Sync>,
        task_manager: Arc<tokio::sync::Mutex<CrawlTaskManager>>,
    ) -> Self {
        Self {
            db,
            config,
            repo,
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
        interaction_repo: Arc<dyn InteractionRepository + Send + Sync>,
        record_repo: Arc<dyn RecordRepository + Send + Sync>,
        file_service: Arc<dyn LunaFileServiceTrait + Send + Sync>,
        task_manager: Arc<tokio::sync::Mutex<CrawlTaskManager>>,
    ) -> Arc<dyn CrawlServiceTrait> {
        Arc::new(Self {
            db,
            config,
            repo: crawl_repo,
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
    ) -> Result<(i64, TaskStatus), AppError> {
        let canonical_codes: Vec<String> = codes.into_iter().map(|c| c.to_uppercase()).collect();
        let total = canonical_codes.len() as i32;

        let input = CrawlTaskInput::Batch(BatchTaskInput {
            codes: canonical_codes.clone(),
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
    ) -> Result<(i64, TaskStatus), AppError> {
        let input = CrawlTaskInput::Auto(AutoTaskInput {
            start_url,
            max_pages,
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
        liked_only: bool,
        created_after: Option<String>,
    ) -> Result<(i64, TaskStatus), AppError> {
        let target_ids = self
            .resolve_update_targets(user_id, liked_only, created_after.as_deref())
            .await?;

        if target_ids.is_empty() {
            return Err(AppError::ValidationError(
                "No records match the specified filters".to_owned(),
            ));
        }

        let filters = UpdateFilters {
            liked_only,
            created_after: created_after.clone(),
        };
        let input = CrawlTaskInput::Update(UpdateTaskInput {
            filters,
            target_ids: target_ids.clone(),
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
            CancelAction::Running => {}
            CancelAction::RemovedFromQueue => {
                drop(mgr);
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

    fn task_manager(&self) -> Arc<tokio::sync::Mutex<CrawlTaskManager>> {
        self.task_manager.clone()
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
