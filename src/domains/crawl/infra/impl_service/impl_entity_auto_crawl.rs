use luneth::crawl::CrawlError;
use tokio_util::sync::CancellationToken;

use crate::domains::crawl::domain::model::{
    EntityAutoCrawlTaskInput, PageResultStatus, TaskStatus,
    ENTITY_AUTO_CRAWL_MAX_CONSECUTIVE_ERRORS,
};
use crate::domains::crawl::domain::service::CrawlerTrait;
use crate::domains::crawl::dto::task_dto::{SseEvent, TaskSummary};

use super::impl_helpers::ProcessResult;
use super::CrawlService;

impl CrawlService {
    /// Execute an entity-auto-crawl task: iterate `{link}/{page}` upward with
    /// no upper bound, stopping on the first 404 (end of listing) or when the
    /// circuit breaker trips (N consecutive non-404 page errors). Each content
    /// page's records are processed via the shared per-code flow.
    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_lines
    )]
    pub async fn execute_entity_auto_crawl_task(
        &self,
        task_id: i64,
        input: EntityAutoCrawlTaskInput,
        user_id: String,
        crawler: &dyn CrawlerTrait,
        cancel_token: CancellationToken,
    ) {
        let _ = self.repo.update_task_started(&self.db, task_id).await;

        let mut success_count = 0i32;
        let mut fail_count = 0i32;
        let mut skip_count = 0i32;
        let mut total_codes = 0i32;

        // Accounting for termination grading.
        let mut reached_content = false;
        let mut content_pages = 0i32;
        let mut error_page_numbers: Vec<i32> = Vec::new();
        let mut consecutive_errors: u32 = 0;
        let mut tripped_breaker = false;
        let mut was_cancelled = false;

        {
            let mgr = self.task_manager.lock().await;
            mgr.emit_event(SseEvent::TaskStarted {
                task_id,
                user_id: user_id.clone(),
                task_type: "entity_auto_crawl".to_owned(),
            });
        }

        let mut page: u32 = 1;
        loop {
            if cancel_token.is_cancelled() {
                was_cancelled = true;
                break;
            }

            // input.link is the trimmed absolute listing URL; pagination is
            // `{link}/{page}`.
            let page_url = format!("{}/{}", input.link, page);

            {
                let mgr = self.task_manager.lock().await;
                mgr.emit_event(SseEvent::PageStart {
                    task_id,
                    user_id: user_id.clone(),
                    page_number: page as i32,
                });
            }

            match crawler.crawl_page(&page_url).await {
                Ok(pieces) => {
                    let records_found = pieces.len() as i32;
                    total_codes += records_found;

                    let _ = self
                        .repo
                        .update_task_counts(
                            &self.db,
                            task_id,
                            success_count,
                            fail_count,
                            skip_count,
                            total_codes,
                        )
                        .await;

                    let page_result = self
                        .repo
                        .create_page_result(
                            &self.db,
                            task_id,
                            page as i32,
                            &PageResultStatus::Processing,
                            records_found,
                            None,
                        )
                        .await;

                    if records_found == 0 {
                        // Empty page: success(0), reset the error counter, keep
                        // going (an empty page is NOT a stop condition).
                        if let Ok(pr) = page_result {
                            let _ = self
                                .repo
                                .update_page_result(
                                    &self.db,
                                    pr.id,
                                    &PageResultStatus::Success,
                                    0,
                                    None,
                                )
                                .await;
                        }
                        consecutive_errors = 0;
                        page += 1;
                        continue;
                    }

                    let mut page_crawled = 0i32;
                    for piece in &pieces {
                        if cancel_token.is_cancelled() {
                            break;
                        }
                        let res = self
                            .process_single_code(
                                task_id,
                                luneth::crawl::CrawlInput::Piece(piece.clone()),
                                false,
                                false,
                                &user_id,
                                crawler,
                            )
                            .await;
                        match res {
                            ProcessResult::Success | ProcessResult::Partial => {
                                success_count += 1;
                                page_crawled += 1;
                            }
                            ProcessResult::Failed => fail_count += 1,
                            ProcessResult::Skipped => skip_count += 1,
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

                    let cancelled_now = cancel_token.is_cancelled();
                    let page_status = if cancelled_now {
                        PageResultStatus::Failed
                    } else {
                        PageResultStatus::Success
                    };
                    if let Ok(pr) = page_result {
                        let _ = self
                            .repo
                            .update_page_result(
                                &self.db,
                                pr.id,
                                &page_status,
                                page_crawled,
                                if cancelled_now {
                                    Some("Page processing interrupted: task cancelled")
                                } else {
                                    None
                                },
                            )
                            .await;
                    }

                    if cancelled_now {
                        was_cancelled = true;
                        break;
                    }

                    reached_content = true;
                    content_pages += 1;
                    consecutive_errors = 0;

                    {
                        let mgr = self.task_manager.lock().await;
                        mgr.emit_event(SseEvent::PageComplete {
                            task_id,
                            user_id: user_id.clone(),
                            page_number: page as i32,
                            records_found,
                            records_crawled: page_crawled,
                        });
                    }

                    page += 1;
                }
                Err(e) => {
                    if matches!(e, CrawlError::PageNotFound { .. }) {
                        // Stop condition (1): first 404 = end of listing. The 404
                        // page is written failed but NOT counted in error_page_numbers.
                        let _ = self
                            .repo
                            .create_page_result(
                                &self.db,
                                task_id,
                                page as i32,
                                &PageResultStatus::Failed,
                                0,
                                Some(&e.to_string()),
                            )
                            .await;
                        break;
                    } else {
                        // Non-404 page error: record the page number, bump the
                        // consecutive counter; trip the breaker if it reaches N.
                        let _ = self
                            .repo
                            .create_page_result(
                                &self.db,
                                task_id,
                                page as i32,
                                &PageResultStatus::Failed,
                                0,
                                Some(&e.to_string()),
                            )
                            .await;
                        error_page_numbers.push(page as i32);
                        consecutive_errors += 1;
                        if consecutive_errors >= ENTITY_AUTO_CRAWL_MAX_CONSECUTIVE_ERRORS {
                            tripped_breaker = true;
                            break;
                        }
                        page += 1;
                    }
                }
            }
        }

        // Clean up any page left in `processing` (only possible on cancel).
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

        // Final status grading.
        let breaker_msg = format!(
            "Circuit breaker: stopped after {ENTITY_AUTO_CRAWL_MAX_CONSECUTIVE_ERRORS} consecutive non-404 page errors"
        );
        let (final_status, error_message): (TaskStatus, Option<&str>) = if was_cancelled {
            (TaskStatus::Cancelled, None)
        } else if tripped_breaker {
            // Begins with "Circuit breaker:" so the detail/summary can
            // distinguish it from a restart-interrupt failure.
            (TaskStatus::Failed, Some(&breaker_msg))
        } else if !reached_content || !error_page_numbers.is_empty() {
            // Broken link (no content page ever succeeded) or a run that hit at
            // least one non-404 page error -> Failed.
            (TaskStatus::Failed, None)
        } else {
            (TaskStatus::Completed, None)
        };

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
                error_message,
            )
            .await;

        // Progress row finalize (Model D). Successful completion is the ONLY
        // event that advances the round: set last_crawled_round =
        // GREATEST(last_crawled_round, crawl_round + 1). Failure and cancellation
        // never touch the round (the entity stays at its round = MIN and is
        // re-selectable via the uncrawled scope); they only stamp last_crawled_at.
        match final_status {
            TaskStatus::Completed => {
                let _ = self
                    .entity_repo
                    .advance_round_on_complete(
                        &self.db,
                        input.entity_type,
                        input.entity_id,
                        input.crawl_round,
                    )
                    .await;
            }
            _ => {
                let _ = self
                    .entity_repo
                    .touch_on_finalize(&self.db, input.entity_type, input.entity_id)
                    .await;
            }
        }

        let pages_crawled = content_pages as i64;
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
                    status: final_status.as_str().to_owned(),
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
