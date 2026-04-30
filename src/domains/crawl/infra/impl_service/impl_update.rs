use luneth::crawl::CrawlInput;
use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};
use tokio_util::sync::CancellationToken;

use crate::common::error::AppError;
use crate::domains::crawl::domain::model::TaskStatus;
use crate::domains::crawl::domain::service::CrawlerTrait;
use crate::domains::crawl::dto::task_dto::{SseEvent, TaskSummary};
use crate::domains::luna::dto::CreateLinkDto;
use crate::entities::{record, user_record_interaction};

use super::CrawlService;

impl CrawlService {
    pub(super) async fn resolve_update_targets(
        &self,
        user_id: &str,
        liked_only: bool,
        created_after: Option<&str>,
    ) -> Result<Vec<String>, AppError> {
        let mut query = record::Entity::find();

        if liked_only {
            query = query.filter(
                record::Column::Id.in_subquery(
                    sea_orm::sea_query::Query::select()
                        .column(user_record_interaction::Column::RecordId)
                        .from(user_record_interaction::Entity)
                        .and_where(
                            sea_orm::sea_query::Expr::col(user_record_interaction::Column::UserId)
                                .eq(user_id),
                        )
                        .and_where(
                            sea_orm::sea_query::Expr::col(user_record_interaction::Column::Liked)
                                .eq(true),
                        )
                        .to_owned(),
                ),
            );
        }

        if let Some(date_str) = created_after {
            let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_e| {
                AppError::ValidationError(format!(
                    "Invalid created_after date format: {date_str}. Expected YYYY-MM-DD"
                ))
            })?;
            query = query.filter(record::Column::CreateTime.gte(date));
        }

        let records = query.all(&self.db).await.map_err(AppError::DatabaseError)?;
        Ok(records.into_iter().map(|r| r.id).collect())
    }

    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_lines
    )]
    pub async fn execute_update_task(
        &self,
        task_id: i64,
        target_ids: Vec<String>,
        user_id: String,
        crawler: &dyn CrawlerTrait,
        cancel_token: CancellationToken,
    ) {
        let _ = self.repo.update_task_started(&self.db, task_id).await;

        let mut success_count = 0i32;
        let mut fail_count = 0i32;
        let mut skip_count = 0i32;
        let total_codes = target_ids.len() as i32;

        {
            let mgr = self.task_manager.lock().await;
            mgr.emit_event(SseEvent::TaskStarted {
                task_id,
                user_id: user_id.clone(),
                task_type: "update".to_owned(),
            });
        }

        for record_id in &target_ids {
            if cancel_token.is_cancelled() {
                break;
            }

            match self
                .record_repo
                .find_by_id(&self.db, record_id.clone())
                .await
            {
                Ok(Some(_)) => {}
                Ok(None) => {
                    fail_count += 1;
                    let _ = self
                        .repo
                        .create_code_result(
                            &self.db,
                            task_id,
                            record_id,
                            "failed",
                            Some(record_id),
                            0,
                            Some("Record no longer exists"),
                        )
                        .await;
                    self.emit_code_progress_async(
                        task_id,
                        &user_id,
                        record_id,
                        "failed",
                        Some(record_id),
                        0,
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
                Err(e) => {
                    fail_count += 1;
                    let _ = self
                        .repo
                        .create_code_result(
                            &self.db,
                            task_id,
                            record_id,
                            "failed",
                            None,
                            0,
                            Some(&format!("DB error: {e}")),
                        )
                        .await;
                    self.emit_code_progress_async(task_id, &user_id, record_id, "failed", None, 0)
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
            }

            match crawler
                .crawl_recorder_with_imgs(CrawlInput::Code(record_id.clone()))
                .await
            {
                Ok((recorder, _images)) => {
                    let new_links: Vec<CreateLinkDto> = recorder
                        .record
                        .share_magnet_links
                        .iter()
                        .filter(|ml| !ml.link.trim().is_empty())
                        .map(|ml| CreateLinkDto {
                            name: if ml.name.is_empty() {
                                "None".to_owned()
                            } else {
                                ml.name.clone()
                            },
                            size: ml.size.parse().ok(),
                            date: ml.date.parse().ok(),
                            link: ml.link.clone(),
                            star: Some(ml.star),
                        })
                        .collect();

                    let changed = self
                        .update_record_links_via_repo(record_id, &new_links)
                        .await;

                    match changed {
                        Ok(true) => {
                            success_count += 1;
                            let _ = self
                                .repo
                                .create_code_result(
                                    &self.db,
                                    task_id,
                                    record_id,
                                    "success",
                                    Some(record_id),
                                    0,
                                    None,
                                )
                                .await;
                            self.emit_code_progress_async(
                                task_id,
                                &user_id,
                                record_id,
                                "success",
                                Some(record_id),
                                0,
                            )
                            .await;
                        }
                        Ok(false) => {
                            skip_count += 1;
                            let _ = self
                                .repo
                                .create_code_result(
                                    &self.db,
                                    task_id,
                                    record_id,
                                    "skipped",
                                    Some(record_id),
                                    0,
                                    None,
                                )
                                .await;
                            self.emit_code_progress_async(
                                task_id,
                                &user_id,
                                record_id,
                                "skipped",
                                Some(record_id),
                                0,
                            )
                            .await;
                        }
                        Err(e) => {
                            fail_count += 1;
                            let _ = self
                                .repo
                                .create_code_result(
                                    &self.db,
                                    task_id,
                                    record_id,
                                    "failed",
                                    Some(record_id),
                                    0,
                                    Some(&e.to_string()),
                                )
                                .await;
                            self.emit_code_progress_async(
                                task_id,
                                &user_id,
                                record_id,
                                "failed",
                                Some(record_id),
                                0,
                            )
                            .await;
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
                            record_id,
                            "failed",
                            Some(record_id),
                            0,
                            Some(&format!("{e}")),
                        )
                        .await;
                    self.emit_code_progress_async(
                        task_id,
                        &user_id,
                        record_id,
                        "failed",
                        Some(record_id),
                        0,
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
                total_codes,
                None,
            )
            .await;

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
                        pages_crawled: 0,
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
                        pages_crawled: 0,
                    },
                });
            }
        }
    }
}
