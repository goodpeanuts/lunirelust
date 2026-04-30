use luneth::crawl::CrawlInput;
use tokio_util::sync::CancellationToken;

use crate::domains::crawl::domain::model::{CrawlTaskInput, TaskStatus};
use crate::domains::crawl::domain::service::CrawlerTrait;
use crate::domains::crawl::dto::task_dto::{SseEvent, TaskSummary};

use super::CrawlService;

impl CrawlService {
    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::manual_let_else
    )]
    pub async fn dispatch_and_run(&self, task_id: i64, crawler: &dyn CrawlerTrait) {
        let task = if let Ok(Some(t)) = self.repo.get_task_by_id(&self.db, task_id).await {
            t
        } else {
            tracing::error!("Task {task_id} disappeared before execution");
            let mut mgr = self.task_manager.lock().await;
            mgr.complete_current();
            return;
        };

        let payload = if let Some(p) = task.input_payload.as_deref() {
            p
        } else {
            tracing::error!("Task {task_id} has no input_payload");
            let _ = self
                .repo
                .complete_task(
                    &self.db,
                    task_id,
                    &TaskStatus::Failed,
                    0,
                    0,
                    0,
                    task.total_codes,
                    Some("Missing input payload"),
                )
                .await;
            self.emit_failed_terminal_async(task_id, &task.user_id, task.total_codes, 0, 0, 0)
                .await;
            let mut mgr = self.task_manager.lock().await;
            mgr.complete_current();
            return;
        };

        let input: CrawlTaskInput = match serde_json::from_str(payload) {
            Ok(i) => i,
            Err(e) => {
                tracing::error!("Task {task_id} has invalid payload: {e}");
                let _ = self
                    .repo
                    .complete_task(
                        &self.db,
                        task_id,
                        &TaskStatus::Failed,
                        0,
                        0,
                        0,
                        task.total_codes,
                        Some("Invalid persisted payload"),
                    )
                    .await;
                self.emit_failed_terminal_async(task_id, &task.user_id, task.total_codes, 0, 0, 0)
                    .await;
                let mut mgr = self.task_manager.lock().await;
                mgr.complete_current();
                return;
            }
        };

        let cancel_token = CancellationToken::new();
        {
            let mut mgr = self.task_manager.lock().await;
            mgr.set_cancellation_token(task_id, cancel_token.clone());
        }

        let user_id = task.user_id.clone();
        match input {
            CrawlTaskInput::Batch(bi) => {
                self.execute_batch_task(
                    task_id,
                    bi.codes,
                    bi.mark_liked,
                    bi.mark_viewed,
                    user_id,
                    crawler,
                    cancel_token,
                )
                .await;
            }
            CrawlTaskInput::Auto(ai) => {
                self.execute_auto_task(
                    task_id,
                    ai.start_url,
                    ai.max_pages,
                    ai.mark_liked,
                    ai.mark_viewed,
                    user_id,
                    crawler,
                    cancel_token,
                )
                .await;
            }
            CrawlTaskInput::Update(ui) => {
                self.execute_update_task(task_id, ui.target_ids, user_id, crawler, cancel_token)
                    .await;
            }
        }

        // Task finished (success or cancellation) — clear current_task and dispatch next.
        {
            let mut mgr = self.task_manager.lock().await;
            mgr.complete_current();
        }
    }

    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_arguments,
        clippy::too_many_lines
    )]
    pub async fn execute_batch_task(
        &self,
        task_id: i64,
        codes: Vec<String>,
        mark_liked: bool,
        mark_viewed: bool,
        user_id: String,
        crawler: &dyn CrawlerTrait,
        cancel_token: CancellationToken,
    ) {
        let _ = self.repo.update_task_started(&self.db, task_id).await;

        let mut success_count = 0i32;
        let mut fail_count = 0i32;
        let mut skip_count = 0i32;
        let total_codes = codes.len() as i32;

        {
            let mgr = self.task_manager.lock().await;
            mgr.emit_event(SseEvent::TaskStarted {
                task_id,
                user_id: user_id.clone(),
                task_type: "batch".to_owned(),
            });
        }

        let mut seen_codes = std::collections::HashSet::new();

        for code in &codes {
            if cancel_token.is_cancelled() {
                break;
            }

            let is_dup_in_request = !seen_codes.insert(code.clone());
            if is_dup_in_request {
                skip_count += 1;
                self.persist_code_result_and_emit_progress(
                    task_id,
                    &user_id,
                    code,
                    "skipped",
                    Some(code),
                    0,
                    Some("Duplicate code in same request"),
                )
                .await;

                self.emit_stats_async(
                    task_id,
                    &user_id,
                    success_count,
                    fail_count,
                    skip_count,
                    total_codes,
                )
                .await;
                continue;
            }

            match self.record_repo.find_by_id(&self.db, code.clone()).await {
                Ok(Some(_existing)) => {
                    if mark_liked {
                        let _ = self
                            .interaction_repo
                            .mark_liked(&self.db, &user_id, code)
                            .await;
                    }
                    if mark_viewed {
                        let _ = self
                            .interaction_repo
                            .mark_viewed(&self.db, &user_id, code)
                            .await;
                    }

                    skip_count += 1;
                    self.persist_code_result_and_emit_progress(
                        task_id,
                        &user_id,
                        code,
                        "skipped",
                        Some(code),
                        0,
                        None,
                    )
                    .await;
                    self.emit_stats_async(
                        task_id,
                        &user_id,
                        success_count,
                        fail_count,
                        skip_count,
                        total_codes,
                    )
                    .await;
                }
                Ok(None) => {
                    match crawler
                        .crawl_recorder_with_imgs(CrawlInput::Code(code.clone()))
                        .await
                    {
                        Ok((recorder, images)) => {
                            match self
                                .insert_crawled_record(&recorder.record, &images, code)
                                .await
                            {
                                Ok((record_id, images_downloaded, is_partial)) => {
                                    if mark_liked {
                                        let _ = self
                                            .interaction_repo
                                            .mark_liked(&self.db, &user_id, &record_id)
                                            .await;
                                    }
                                    if mark_viewed {
                                        let _ = self
                                            .interaction_repo
                                            .mark_viewed(&self.db, &user_id, &record_id)
                                            .await;
                                    }

                                    let status_str = if is_partial { "partial" } else { "success" };
                                    success_count += 1;

                                    self.persist_code_result_and_emit_progress(
                                        task_id,
                                        &user_id,
                                        code,
                                        status_str,
                                        Some(&record_id),
                                        images_downloaded,
                                        None,
                                    )
                                    .await;
                                }
                                Err(e) => {
                                    fail_count += 1;
                                    self.persist_code_result_and_emit_progress(
                                        task_id,
                                        &user_id,
                                        code,
                                        "failed",
                                        None,
                                        0,
                                        Some(&e.to_string()),
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(e) => {
                            fail_count += 1;
                            self.persist_code_result_and_emit_progress(
                                task_id,
                                &user_id,
                                code,
                                "failed",
                                None,
                                0,
                                Some(&format!("{e}")),
                            )
                            .await;
                        }
                    }
                    self.emit_stats_async(
                        task_id,
                        &user_id,
                        success_count,
                        fail_count,
                        skip_count,
                        total_codes,
                    )
                    .await;
                }
                Err(e) => {
                    fail_count += 1;
                    self.persist_code_result_and_emit_progress(
                        task_id,
                        &user_id,
                        code,
                        "failed",
                        None,
                        0,
                        Some(&format!("DB error: {e}")),
                    )
                    .await;
                    self.emit_stats_async(
                        task_id,
                        &user_id,
                        success_count,
                        fail_count,
                        skip_count,
                        total_codes,
                    )
                    .await;
                }
            }
        }

        let was_cancelled = cancel_token.is_cancelled();
        let final_status = if was_cancelled {
            TaskStatus::Cancelled
        } else {
            TaskStatus::Completed
        };

        if was_cancelled {
            let _ = self
                .repo
                .fail_processing_page_results(
                    &self.db,
                    task_id,
                    "Page processing interrupted: task cancelled",
                )
                .await;
        }

        let _ = self
            .repo
            .complete_task(
                &self.db,
                task_id,
                &final_status,
                success_count,
                fail_count,
                skip_count,
                total_codes,
                None,
            )
            .await;

        let pages_crawled = self.count_successful_pages(task_id).await;

        {
            let mgr = self.task_manager.lock().await;
            if was_cancelled {
                mgr.emit_event(SseEvent::TaskCancelled {
                    task_id,
                    user_id: user_id.clone(),
                    summary: TaskSummary {
                        total: total_codes,
                        success: success_count,
                        failed: fail_count,
                        skipped: skip_count,
                        pages_crawled,
                    },
                });
            } else {
                mgr.emit_event(SseEvent::TaskCompleted {
                    task_id,
                    user_id: user_id.clone(),
                    status: "completed".to_owned(),
                    summary: TaskSummary {
                        total: total_codes,
                        success: success_count,
                        failed: fail_count,
                        skipped: skip_count,
                        pages_crawled,
                    },
                });
            }
        }
    }
}
