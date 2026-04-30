use tokio_util::sync::CancellationToken;

use crate::domains::crawl::domain::model::{PageResultStatus, TaskStatus};
use crate::domains::crawl::domain::service::CrawlerTrait;
use crate::domains::crawl::dto::task_dto::{SseEvent, TaskSummary};

use super::impl_helpers::ProcessResult;
use super::CrawlService;

impl CrawlService {
    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_arguments,
        clippy::too_many_lines
    )]
    pub async fn execute_auto_task(
        &self,
        task_id: i64,
        start_url: String,
        max_pages: u32,
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
        let mut total_codes = 0i32;

        {
            let mgr = self.task_manager.lock().await;
            mgr.emit_event(SseEvent::TaskStarted {
                task_id,
                user_id: user_id.clone(),
                task_type: "auto".to_owned(),
            });
        }

        for page_num in 1..=max_pages {
            if cancel_token.is_cancelled() {
                break;
            }

            let page_url = format!("{start_url}/page/{page_num}");

            {
                let mgr = self.task_manager.lock().await;
                mgr.emit_event(SseEvent::PageStart {
                    task_id,
                    user_id: user_id.clone(),
                    page_number: page_num as i32,
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
                            page_num as i32,
                            &PageResultStatus::Processing,
                            records_found,
                            None,
                        )
                        .await;

                    if records_found == 0 {
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
                        break;
                    }

                    let mut page_crawled = 0i32;
                    for piece in &pieces {
                        if cancel_token.is_cancelled() {
                            break;
                        }

                        let code = piece.code.to_uppercase();
                        match self
                            .process_single_code(
                                task_id,
                                &code,
                                mark_liked,
                                mark_viewed,
                                &user_id,
                                crawler,
                            )
                            .await
                        {
                            ProcessResult::Success | ProcessResult::Partial => {
                                success_count += 1;
                                page_crawled += 1;
                            }
                            ProcessResult::Failed => {
                                fail_count += 1;
                            }
                            ProcessResult::Skipped => {
                                skip_count += 1;
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

                    let page_status = if cancel_token.is_cancelled() {
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
                                if cancel_token.is_cancelled() {
                                    Some("Page processing interrupted: task cancelled")
                                } else {
                                    None
                                },
                            )
                            .await;
                    }

                    if matches!(page_status, PageResultStatus::Success) {
                        let mgr = self.task_manager.lock().await;
                        mgr.emit_event(SseEvent::PageComplete {
                            task_id,
                            user_id: user_id.clone(),
                            page_number: page_num as i32,
                            records_found,
                            records_crawled: page_crawled,
                        });
                    }
                }
                Err(e) => {
                    let _ = self
                        .repo
                        .create_page_result(
                            &self.db,
                            task_id,
                            page_num as i32,
                            &PageResultStatus::Failed,
                            0,
                            Some(&format!("{e}")),
                        )
                        .await;
                    tracing::warn!("Auto crawl page {page_num} failed: {e}");
                }
            }
        }

        let was_cancelled = cancel_token.is_cancelled();
        let final_status = if was_cancelled {
            TaskStatus::Cancelled
        } else {
            TaskStatus::Completed
        };

        let _ = self
            .repo
            .fail_processing_page_results(
                &self.db,
                task_id,
                if was_cancelled {
                    "Page processing interrupted: task cancelled"
                } else {
                    "Page processing interrupted: task completed"
                },
            )
            .await;

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
