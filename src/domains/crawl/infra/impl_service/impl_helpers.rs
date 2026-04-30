use chrono::Utc;
use luneth::common::ImageData;
use luneth::crawl::CrawlInput;
use luneth::record::RecordEntry;
use sea_orm::{
    ActiveModelTrait as _, DatabaseConnection, EntityTrait as _, Set, TransactionTrait as _,
};

use crate::common::error::AppError;
use crate::domains::crawl::domain::model::{PageResultStatus, TaskStatus};
use crate::domains::crawl::domain::service::CrawlerTrait;
use crate::domains::crawl::dto::task_dto::{SseEvent, TaskSummary};
use crate::domains::luna::dto::{
    CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateLinkDto,
    CreateRecordDto, CreateSeriesDto, CreateStudioDto,
};
use crate::domains::luna::outbox_entity_upsert;
use crate::domains::search::{
    OutboxRepo, OutboxRepository as _, SearchEntityType, TombstoneRepo, TombstoneRepository as _,
};
use crate::entities::record;

use super::CrawlService;

pub(super) enum ProcessResult {
    Success,
    Partial,
    Failed,
    Skipped,
}

impl CrawlService {
    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_lines
    )]
    pub(super) async fn process_single_code(
        &self,
        task_id: i64,
        code: &str,
        mark_liked: bool,
        mark_viewed: bool,
        user_id: &str,
        crawler: &dyn CrawlerTrait,
    ) -> ProcessResult {
        match self.record_repo.find_by_id(&self.db, code.to_owned()).await {
            Ok(Some(_)) => {
                if mark_liked {
                    let _ = self
                        .interaction_repo
                        .mark_liked(&self.db, user_id, code)
                        .await;
                }
                if mark_viewed {
                    let _ = self
                        .interaction_repo
                        .mark_viewed(&self.db, user_id, code)
                        .await;
                }
                let _ = self
                    .repo
                    .create_code_result(&self.db, task_id, code, "skipped", Some(code), 0, None)
                    .await;
                self.emit_code_progress_async(task_id, user_id, code, "skipped", Some(code), 0)
                    .await;
                ProcessResult::Skipped
            }
            Ok(None) => {
                match crawler
                    .crawl_recorder_with_imgs(CrawlInput::Code(code.to_owned()))
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
                                        .mark_liked(&self.db, user_id, &record_id)
                                        .await;
                                }
                                if mark_viewed {
                                    let _ = self
                                        .interaction_repo
                                        .mark_viewed(&self.db, user_id, &record_id)
                                        .await;
                                }

                                let status_str = if is_partial { "partial" } else { "success" };
                                let _ = self
                                    .repo
                                    .create_code_result(
                                        &self.db,
                                        task_id,
                                        code,
                                        status_str,
                                        Some(&record_id),
                                        images_downloaded,
                                        None,
                                    )
                                    .await;
                                self.emit_code_progress_async(
                                    task_id,
                                    user_id,
                                    code,
                                    status_str,
                                    Some(&record_id),
                                    images_downloaded,
                                )
                                .await;

                                if is_partial {
                                    ProcessResult::Partial
                                } else {
                                    ProcessResult::Success
                                }
                            }
                            Err(e) => {
                                let _ = self
                                    .repo
                                    .create_code_result(
                                        &self.db,
                                        task_id,
                                        code,
                                        "failed",
                                        None,
                                        0,
                                        Some(&e.to_string()),
                                    )
                                    .await;
                                self.emit_code_progress_async(
                                    task_id, user_id, code, "failed", None, 0,
                                )
                                .await;
                                ProcessResult::Failed
                            }
                        }
                    }
                    Err(e) => {
                        let _ = self
                            .repo
                            .create_code_result(
                                &self.db,
                                task_id,
                                code,
                                "failed",
                                None,
                                0,
                                Some(&format!("{e}")),
                            )
                            .await;
                        self.emit_code_progress_async(task_id, user_id, code, "failed", None, 0)
                            .await;
                        ProcessResult::Failed
                    }
                }
            }
            Err(e) => {
                let _ = self
                    .repo
                    .create_code_result(
                        &self.db,
                        task_id,
                        code,
                        "failed",
                        None,
                        0,
                        Some(&format!("DB error: {e}")),
                    )
                    .await;
                self.emit_code_progress_async(task_id, user_id, code, "failed", None, 0)
                    .await;
                ProcessResult::Failed
            }
        }
    }

    /// Map a luneth `RecordEntry` to `CreateRecordDto` and insert via record repo
    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_lines
    )]
    pub(super) async fn insert_crawled_record(
        &self,
        entry: &RecordEntry,
        images: &[ImageData],
        _code: &str,
    ) -> Result<(String, i32, bool), AppError> {
        let links: Vec<CreateLinkDto> = entry
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

        let has_links = !links.is_empty();

        let director = entry
            .director
            .iter()
            .next()
            .map(|(name, lnk)| CreateDirectorDto {
                name: name.clone(),
                link: Some(lnk.clone()),
                manual: None,
            });
        let studio = entry
            .studio
            .iter()
            .next()
            .map(|(name, lnk)| CreateStudioDto {
                name: name.clone(),
                link: Some(lnk.clone()),
                manual: None,
            });
        let label = entry.label.iter().next().map(|(name, lnk)| CreateLabelDto {
            name: name.clone(),
            link: Some(lnk.clone()),
            manual: None,
        });
        let series = entry
            .series
            .iter()
            .next()
            .map(|(name, lnk)| CreateSeriesDto {
                name: name.clone(),
                link: Some(lnk.clone()),
                manual: None,
            });

        let genres: Vec<CreateGenreDto> = entry
            .genre
            .values()
            .map(|name| CreateGenreDto {
                name: name.clone(),
                link: None,
                manual: None,
            })
            .collect();

        let idols: Vec<CreateIdolDto> = entry
            .idols
            .iter()
            .map(|(name, lnk)| CreateIdolDto {
                name: name.clone(),
                link: Some(lnk.clone()),
                manual: None,
            })
            .collect();

        let date = chrono::NaiveDate::parse_from_str(&entry.release_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Utc::now().date_naive());

        let duration: i32 = entry.length.parse().unwrap_or(0);

        let create_dto = CreateRecordDto {
            id: entry.id.clone(),
            title: entry.title.clone(),
            date,
            duration,
            director,
            studio,
            label,
            series,
            genres,
            idols,
            has_links,
            links,
            permission: 0,
            local_img_count: 0,
            creator: "crawl".to_owned(),
            modified_by: "crawl".to_owned(),
        };

        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let (id, nested) = match self.record_repo.create(&txn, create_dto).await {
            Ok(result) => result,
            Err(e) => {
                let _ = txn.rollback().await;
                return Err(AppError::DatabaseError(e));
            }
        };

        if let Err(e) = self.insert_nested_outbox_events(&txn, &nested).await {
            let _ = txn.rollback().await;
            return Err(e);
        }

        let version = Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_millis() * 1_000_000);
        if let Err(e) =
            OutboxRepo::insert_event(&txn, "record", &id, "upsert", version, None, None).await
        {
            let _ = txn.rollback().await;
            return Err(AppError::DatabaseError(e));
        }
        if let Err(e) = TombstoneRepo::upsert_version(&txn, "record", &id, version).await {
            let _ = txn.rollback().await;
            return Err(AppError::DatabaseError(e));
        }

        txn.commit().await.map_err(AppError::DatabaseError)?;

        // Save images
        let images_downloaded = self.save_images(&id, images).await;
        let expected_images = images.len() as i32;
        let is_partial = images_downloaded < expected_images;

        // Update local_img_count via direct SeaORM
        if images_downloaded > 0 {
            let _ = Self::update_local_img_count_direct(&self.db, &id, images_downloaded).await;
        }

        Ok((id, images_downloaded, is_partial))
    }

    /// Delegate link update to the Luna `RecordRepository`.
    /// Returns true if any links were changed (inserted or backfilled).
    pub(super) async fn update_record_links_via_repo(
        &self,
        record_id: &str,
        new_links: &[CreateLinkDto],
    ) -> Result<bool, AppError> {
        if new_links.is_empty() {
            return Ok(false);
        }
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;
        let changed = match self
            .record_repo
            .update_record_links(&txn, record_id.to_owned(), new_links.to_vec())
            .await
        {
            Ok(c) => c,
            Err(e) => {
                txn.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };
        txn.commit().await.map_err(AppError::DatabaseError)?;
        Ok(changed > 0)
    }

    pub(super) async fn save_images(&self, record_id: &str, images: &[ImageData]) -> i32 {
        let mut count = 0i32;
        for img in images {
            match img.save_to_dir_path(
                format!(
                    "{}/images/record/{}",
                    self.config.assets_private_path, record_id
                ),
                None,
            ) {
                Ok(_) => count += 1,
                Err(e) => {
                    tracing::warn!("Failed to save image for record {record_id}: {e}");
                }
            }
        }
        count
    }

    pub(super) async fn insert_nested_outbox_events(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        nested: &crate::domains::luna::CreatedNestedEntities,
    ) -> Result<(), AppError> {
        for (entity_type, entity_info) in [
            (SearchEntityType::Director, &nested.director),
            (SearchEntityType::Studio, &nested.studio),
            (SearchEntityType::Label, &nested.label),
            (SearchEntityType::Series, &nested.series),
        ] {
            if let Some((entity_id, entity_name)) = entity_info {
                outbox_entity_upsert(txn, entity_type, *entity_id, entity_name, vec![])
                    .await
                    .map_err(AppError::DatabaseError)?;
            }
        }

        for (genre_id, genre_name) in &nested.genres {
            outbox_entity_upsert(txn, SearchEntityType::Genre, *genre_id, genre_name, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
        }

        for (idol_id, idol_name) in &nested.idols {
            outbox_entity_upsert(txn, SearchEntityType::Idol, *idol_id, idol_name, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
        }

        Ok(())
    }

    pub(super) async fn count_successful_pages(&self, task_id: i64) -> i64 {
        self.repo
            .list_page_results(&self.db, task_id)
            .await
            .map(|pages| {
                pages
                    .iter()
                    .filter(|p| p.status == PageResultStatus::Success)
                    .count() as i64
            })
            .unwrap_or(0)
    }

    #[expect(clippy::let_underscore_must_use, clippy::let_underscore_untyped)]
    pub(super) async fn emit_stats_async(
        &self,
        task_id: i64,
        user_id: &str,
        success: i32,
        fail: i32,
        skip: i32,
        total: i32,
    ) {
        let _ = self
            .repo
            .update_task_counts(&self.db, task_id, success, fail, skip, total)
            .await;

        let mgr = self.task_manager.lock().await;
        mgr.emit_event(SseEvent::Stats {
            task_id,
            user_id: user_id.to_owned(),
            success_count: success,
            fail_count: fail,
            skip_count: skip,
            total,
        });
    }

    pub(super) async fn emit_code_progress_async(
        &self,
        task_id: i64,
        user_id: &str,
        code: &str,
        status: &str,
        record_id: Option<&str>,
        images_downloaded: i32,
    ) {
        let mgr = self.task_manager.lock().await;
        mgr.emit_event(SseEvent::CodeProgress {
            task_id,
            user_id: user_id.to_owned(),
            code: code.to_owned(),
            status: status.to_owned(),
            record_id: record_id.map(|s| s.to_owned()),
            images_downloaded,
        });
    }

    #[expect(
        clippy::let_underscore_must_use,
        clippy::let_underscore_untyped,
        clippy::too_many_arguments
    )]
    pub(super) async fn persist_code_result_and_emit_progress(
        &self,
        task_id: i64,
        user_id: &str,
        code: &str,
        status: &str,
        record_id: Option<&str>,
        images_downloaded: i32,
        error_message: Option<&str>,
    ) {
        let _ = self
            .repo
            .create_code_result(
                &self.db,
                task_id,
                code,
                status,
                record_id,
                images_downloaded,
                error_message,
            )
            .await;

        self.emit_code_progress_async(task_id, user_id, code, status, record_id, images_downloaded)
            .await;
    }

    pub(super) async fn emit_failed_terminal_async(
        &self,
        task_id: i64,
        user_id: &str,
        total: i32,
        success: i32,
        failed: i32,
        skipped: i32,
    ) {
        let pages_crawled = self.count_successful_pages(task_id).await;
        let mgr = self.task_manager.lock().await;
        mgr.emit_event(SseEvent::TaskCompleted {
            task_id,
            user_id: user_id.to_owned(),
            status: TaskStatus::Failed.as_str().to_owned(),
            summary: TaskSummary {
                total,
                success,
                failed,
                skipped,
                pages_crawled,
            },
        });
    }

    pub(super) async fn update_local_img_count_direct(
        db: &DatabaseConnection,
        record_id: &str,
        count: i32,
    ) -> Result<(), AppError> {
        let result = record::Entity::find_by_id(record_id)
            .one(db)
            .await
            .map_err(AppError::DatabaseError)?;

        if let Some(model) = result {
            let mut active: record::ActiveModel = model.into();
            active.local_img_count = Set(count);
            active.update(db).await.map_err(AppError::DatabaseError)?;
        }

        Ok(())
    }
}
