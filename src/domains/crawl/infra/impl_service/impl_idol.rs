use std::path::Path;
use std::time::Duration;

use rand::Rng as _;
use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};
use tokio_util::sync::CancellationToken;

use crate::common::error::AppError;
use crate::domains::crawl::domain::model::{IdolCrawlTarget, TaskStatus};
use crate::domains::crawl::domain::service::CrawlerTrait;
use crate::domains::crawl::dto::task_dto::{SseEvent, TaskSummary};
use crate::entities::idol;

use super::CrawlService;

impl CrawlService {
    pub(super) async fn resolve_idols_without_images(
        &self,
    ) -> Result<Vec<IdolCrawlTarget>, AppError> {
        let all_idols = idol::Entity::find()
            .filter(idol::Column::Id.gt(0))
            .all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        let mut targets = Vec::new();
        for m in all_idols {
            let idol_dir = Path::new(&self.config.assets_private_path)
                .join("images")
                .join("idol")
                .join(&m.name);

            let has_images = idol_dir.exists()
                && std::fs::read_dir(&idol_dir)
                    .map(|mut entries| {
                        entries.any(|e| {
                            e.ok()
                                .and_then(|entry| {
                                    let p = entry.path();
                                    if p.is_file() {
                                        p.extension().and_then(|ext| ext.to_str()).map(|ext| {
                                            self.config
                                                .asset_allowed_extensions
                                                .contains(&ext.to_lowercase())
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);

            if !has_images {
                targets.push(IdolCrawlTarget {
                    id: m.id,
                    name: m.name,
                    link: m.link,
                });
            }
        }

        Ok(targets)
    }

    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_lines
    )]
    pub(super) async fn execute_idol_task(
        &self,
        task_id: i64,
        idols: Vec<IdolCrawlTarget>,
        user_id: String,
        crawler: &dyn CrawlerTrait,
        cancel_token: CancellationToken,
    ) {
        let _ = self.repo.update_task_started(&self.db, task_id).await;

        let mut success_count = 0i32;
        let mut fail_count = 0i32;
        let skip_count = 0;
        let total = idols.len() as i32;

        {
            let mgr = self.task_manager.lock().await;
            mgr.emit_event(SseEvent::TaskStarted {
                task_id,
                user_id: user_id.clone(),
                task_type: "idol".to_owned(),
            });
        }

        for target in &idols {
            if cancel_token.is_cancelled() {
                break;
            }

            let delay = rand::rng().random_range(3..=6);
            tokio::time::sleep(Duration::from_secs(delay)).await;

            if cancel_token.is_cancelled() {
                break;
            }

            match crawler.crawl_idol_image(&target.link).await {
                Ok(image_data) => {
                    let downloaded = self.save_idol_image(&target.name, &image_data);

                    if downloaded > 0 {
                        success_count += 1;
                        let _ = self
                            .repo
                            .create_code_result(
                                &self.db,
                                task_id,
                                &target.name,
                                "success",
                                None,
                                downloaded,
                                None,
                            )
                            .await;
                        {
                            let mgr = self.task_manager.lock().await;
                            mgr.emit_event(SseEvent::IdolProgress {
                                task_id,
                                user_id: user_id.clone(),
                                idol_id: target.id,
                                idol_name: target.name.clone(),
                                status: "success".to_owned(),
                                images_downloaded: downloaded,
                            });
                        }
                    } else {
                        fail_count += 1;
                        let _ = self
                            .repo
                            .create_code_result(
                                &self.db,
                                task_id,
                                &target.name,
                                "failed",
                                None,
                                0,
                                Some("Failed to save image"),
                            )
                            .await;
                        {
                            let mgr = self.task_manager.lock().await;
                            mgr.emit_event(SseEvent::IdolProgress {
                                task_id,
                                user_id: user_id.clone(),
                                idol_id: target.id,
                                idol_name: target.name.clone(),
                                status: "failed".to_owned(),
                                images_downloaded: 0,
                            });
                        }
                    }
                }
                Err(e) => {
                    fail_count += 1;
                    let _ = self
                        .repo
                        .create_code_result(
                            &self.db,
                            task_id,
                            &target.name,
                            "failed",
                            None,
                            0,
                            Some(&e.to_string()),
                        )
                        .await;
                    {
                        let mgr = self.task_manager.lock().await;
                        mgr.emit_event(SseEvent::IdolProgress {
                            task_id,
                            user_id: user_id.clone(),
                            idol_id: target.id,
                            idol_name: target.name.clone(),
                            status: "failed".to_owned(),
                            images_downloaded: 0,
                        });
                    }
                }
            }

            self.emit_stats_async(
                task_id,
                &user_id,
                success_count,
                fail_count,
                skip_count,
                total,
            )
            .await;
        }

        let was_cancelled = cancel_token.is_cancelled();
        let final_status = if was_cancelled {
            TaskStatus::Cancelled
        } else {
            TaskStatus::Completed
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
                total,
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
                        total,
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
                        total,
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
