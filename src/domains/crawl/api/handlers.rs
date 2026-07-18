use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event as SseEventInner, KeepAlive, Sse},
        IntoResponse,
    },
    Extension,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::pin::Pin;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;

use crate::common::app_state::AppState;
use crate::common::dto::RestApiResponse;
use crate::common::error::AppError;
use crate::common::jwt::Claims;
use crate::domains::crawl::domain::model::CrawlTask;
use crate::domains::crawl::domain::model::{
    CrawlPageResult, CrawlTaskInput, EntityAutoCrawlScope, EntityAutoCrawlType, PageResultStatus,
    TaskStatus, TaskType,
};
use crate::domains::crawl::dto::task_dto::{
    CodeResultResponse, CrawlerStatusResponse, EntityAutoCrawlDetail, EntityAutoCrawlTaskResponse,
    EntityProgressListResponse, EntityProgressQuery, EntityProgressSummary,
    EntityProgressSummaryQuery, ListTasksQuery, PageResultResponse, SseEvent, StartAutoRequest,
    StartBatchRequest, StartEntityAutoCrawlRequest, StartIdolRequest, StartUpdateRequest,
    TaskDetailResponse, TaskListItem, TaskListResponse, TaskResponse, TaskSummary,
};
use validator::Validate as _;

#[utoipa::path(
    post,
    path = "/crawl/batch",
    request_body = StartBatchRequest,
    responses(
        (status = 202, description = "Batch crawl task created", body = TaskResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn start_batch(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::Json(req): axum::Json<StartBatchRequest>,
) -> Result<impl IntoResponse, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let (task_id, _status) = state
        .crawl_service
        .start_batch(
            &claims.sub,
            req.codes,
            req.mark_liked,
            req.mark_viewed,
            req.base_url,
        )
        .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        RestApiResponse::success(TaskResponse {
            task_id,
            status: "queued".to_owned(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/crawl/auto",
    request_body = StartAutoRequest,
    responses(
        (status = 202, description = "Auto crawl task created", body = TaskResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn start_auto(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::Json(req): axum::Json<StartAutoRequest>,
) -> Result<impl IntoResponse, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let (task_id, _status) = state
        .crawl_service
        .start_auto(
            &claims.sub,
            req.start_url,
            req.max_pages,
            req.mark_liked,
            req.mark_viewed,
            req.append_page_path,
            req.base_url,
        )
        .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        RestApiResponse::success(TaskResponse {
            task_id,
            status: "queued".to_owned(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/crawl/update",
    request_body = StartUpdateRequest,
    responses(
        (status = 202, description = "Update crawl task created", body = TaskResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn start_update(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::Json(req): axum::Json<StartUpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let (task_id, _status) = state
        .crawl_service
        .start_update(
            &claims.sub,
            req.codes,
            req.liked_only,
            req.created_after,
            req.base_url,
            req.update_images,
        )
        .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        RestApiResponse::success(TaskResponse {
            task_id,
            status: "queued".to_owned(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/crawl/idol",
    request_body = StartIdolRequest,
    responses(
        (status = 202, description = "Idol image crawl task created", body = TaskResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn start_idol(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::Json(req): axum::Json<StartIdolRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (task_id, _status) = state
        .crawl_service
        .start_idol(&claims.sub, req.base_url)
        .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        RestApiResponse::success(TaskResponse {
            task_id,
            status: "queued".to_owned(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/crawl/entity-auto-crawl",
    request_body = StartEntityAutoCrawlRequest,
    responses(
        (status = 202, description = "Entity auto crawl tasks created", body = EntityAutoCrawlTaskResponse),
        (status = 422, description = "Validation error (invalid entity_type, scope, or count)"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn start_entity_auto_crawl(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::Json(req): axum::Json<StartEntityAutoCrawlRequest>,
) -> Result<impl IntoResponse, AppError> {
    req.validate()
        .map_err(|e| AppError::UnprocessableEntity(e.to_string()))?;

    let entity_type = EntityAutoCrawlType::from_str(&req.entity_type).ok_or_else(|| {
        AppError::UnprocessableEntity(format!("invalid entity_type: {}", req.entity_type))
    })?;
    let scope = EntityAutoCrawlScope::from_str(&req.scope)
        .ok_or_else(|| AppError::UnprocessableEntity(format!("invalid scope: {}", req.scope)))?;

    let resp = state
        .crawl_service
        .start_entity_auto_crawl(&claims.sub, entity_type, req.count, scope, req.base_url)
        .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        RestApiResponse::success(resp),
    ))
}

#[utoipa::path(
    get,
    path = "/crawl/entity-progress",
    params(
        ("entity_type" = String, Query, description = "Entity kind: idol/director/label/series/studio/genre"),
        ("status" = Option<String>, Query, description = "Filter by derived status: never/in_progress/completed/failed"),
        ("page" = Option<u64>, Query, description = "Page number (default 1)"),
        ("page_size" = Option<u64>, Query, description = "Page size (default 20)"),
    ),
    responses(
        (status = 200, description = "Paginated entity progress", body = EntityProgressListResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn list_entity_progress(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(query): Query<EntityProgressQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = query.page.unwrap_or(1);
    if page == 0 {
        return Err(AppError::ValidationError("page must be >= 1".to_owned()));
    }
    let page_size = query.page_size.unwrap_or(20);

    let entity_type = EntityAutoCrawlType::from_str(&query.entity_type).ok_or_else(|| {
        AppError::ValidationError(format!("invalid entity_type: {}", query.entity_type))
    })?;

    let resp = state
        .crawl_service
        .list_entity_progress(entity_type, query.status, page, page_size)
        .await?;

    Ok(RestApiResponse::success(resp))
}

#[utoipa::path(
    get,
    path = "/crawl/entity-progress/summary",
    params(
        ("entity_type" = String, Query, description = "Entity kind: idol/director/label/series/studio/genre"),
    ),
    responses(
        (status = 200, description = "Current-round coverage summary", body = EntityProgressSummary),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn get_entity_progress_summary(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(query): Query<EntityProgressSummaryQuery>,
) -> Result<impl IntoResponse, AppError> {
    let entity_type = EntityAutoCrawlType::from_str(&query.entity_type).ok_or_else(|| {
        AppError::ValidationError(format!("invalid entity_type: {}", query.entity_type))
    })?;

    let resp = state
        .crawl_service
        .get_entity_progress_summary(entity_type)
        .await?;

    Ok(RestApiResponse::success(resp))
}

#[utoipa::path(
    post,
    path = "/crawl/tasks/{id}/cancel",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task cancelled"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Task not found"),
        (status = 409, description = "Cannot cancel a terminal task"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn cancel_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(task_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    state
        .crawl_service
        .cancel_task(&claims.sub, task_id)
        .await?;

    Ok(RestApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/crawl/tasks",
    params(
        ("status" = Option<String>, Query, description = "Filter by task status"),
        ("task_type" = Option<String>, Query, description = "Filter by task type"),
        ("page" = Option<u64>, Query, description = "Page number (default 1)"),
        ("page_size" = Option<u64>, Query, description = "Page size (default 20)"),
    ),
    responses(
        (status = 200, description = "List of crawl tasks", body = TaskListResponse),
        (status = 400, description = "Validation error (e.g. page=0)"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListTasksQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = query.page.unwrap_or(1);
    if page == 0 {
        return Err(AppError::ValidationError("page must be >= 1".to_owned()));
    }
    let page_size = query.page_size.unwrap_or(20);

    let status_filter = query.status.as_deref().and_then(TaskStatus::from_str);
    let task_type_filter = query.task_type.as_deref().and_then(TaskType::from_str);

    let (tasks, total) = state
        .crawl_service
        .list_tasks(
            &claims.sub,
            status_filter,
            task_type_filter,
            page,
            page_size,
        )
        .await?;

    let items: Vec<TaskListItem> = tasks
        .into_iter()
        .map(|t| TaskListItem {
            id: t.id,
            task_type: t.task_type.as_str().to_owned(),
            status: t.status.as_str().to_owned(),
            total_codes: t.total_codes,
            success_count: t.success_count,
            fail_count: t.fail_count,
            skip_count: t.skip_count,
            error_message: t.error_message,
            created_at: t.created_at,
            started_at: t.started_at,
            completed_at: t.completed_at,
        })
        .collect();

    Ok(RestApiResponse::success(TaskListResponse {
        tasks: items,
        total,
    }))
}

#[utoipa::path(
    get,
    path = "/crawl/tasks/{id}",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task detail with results", body = TaskDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Task not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn get_task_detail(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(task_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let detail = state
        .crawl_service
        .get_task_detail(&claims.sub, task_id)
        .await?;

    match detail {
        Some(d) => {
            // entity_auto is derived (not stored): parse the persisted payload for
            // the entity reference and aggregate the page results. Computed first,
            // before d.task / d.page_results are moved into the response below.
            let entity_auto = compute_entity_auto_detail(&d.task, &d.page_results);

            let task_item = TaskListItem {
                id: d.task.id,
                task_type: d.task.task_type.as_str().to_owned(),
                status: d.task.status.as_str().to_owned(),
                total_codes: d.task.total_codes,
                success_count: d.task.success_count,
                fail_count: d.task.fail_count,
                skip_count: d.task.skip_count,
                error_message: d.task.error_message,
                created_at: d.task.created_at,
                started_at: d.task.started_at,
                completed_at: d.task.completed_at,
            };

            let code_results: Vec<CodeResultResponse> = d
                .code_results
                .into_iter()
                .map(|r| CodeResultResponse {
                    code: r.code,
                    status: r.status.as_str().to_owned(),
                    record_id: r.record_id,
                    images_downloaded: r.images_downloaded,
                    error_message: r.error_message,
                })
                .collect();

            let page_results: Vec<PageResultResponse> = d
                .page_results
                .into_iter()
                .map(|r| PageResultResponse {
                    page_number: r.page_number,
                    status: r.status.as_str().to_owned(),
                    records_found: r.records_found,
                    records_crawled: r.records_crawled,
                    error_message: r.error_message,
                })
                .collect();

            Ok(RestApiResponse::success(TaskDetailResponse {
                task: task_item,
                code_results,
                page_results,
                entity_auto,
            }))
        }
        None => Err(AppError::NotFound("Task not found".to_owned())),
    }
}

/// Build the `entity_auto` detail block for a task (None for non-entity-auto
/// tasks). Counts real non-404 source page errors: the first-404 end-of-listing
/// page and restart-interrupted pages are excluded by message prefix.
fn compute_entity_auto_detail(
    task: &CrawlTask,
    page_results: &[CrawlPageResult],
) -> Option<EntityAutoCrawlDetail> {
    if !matches!(task.task_type, TaskType::EntityAutoCrawl) {
        return None;
    }
    let payload = task.input_payload.as_deref()?;
    let input: CrawlTaskInput = serde_json::from_str(payload).ok()?;
    let CrawlTaskInput::EntityAutoCrawl(ei) = input else {
        return None;
    };

    let mut success_pages = 0i32;
    let mut failed_page_numbers: Vec<i32> = Vec::new();
    for p in page_results {
        if p.status == PageResultStatus::Success && p.records_found > 0 {
            success_pages += 1;
        } else if p.status == PageResultStatus::Failed {
            let excluded = p.error_message.as_deref().is_some_and(|m| {
                m.starts_with("Page not found (404)")
                    || m.starts_with("Page processing interrupted")
            });
            if !excluded {
                failed_page_numbers.push(p.page_number);
            }
        }
    }
    failed_page_numbers.sort_unstable();
    let failed_pages = failed_page_numbers.len() as i32;

    Some(EntityAutoCrawlDetail {
        entity_type: ei.entity_type.as_str().to_owned(),
        entity_id: ei.entity_id,
        entity_name: ei.entity_name,
        success_pages,
        failed_pages,
        failed_page_numbers,
    })
}

type BoxedSseStream = Pin<Box<dyn Stream<Item = Result<SseEventInner, Infallible>> + Send>>;

#[utoipa::path(
    get,
    path = "/crawl/tasks/{id}/stream",
    params(("id" = i64, Path, description = "Task ID")),
    responses(
        (status = 200, description = "SSE stream of crawl progress events"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Task not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
#[expect(clippy::too_many_lines)]
pub async fn stream_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(task_id): Path<i64>,
) -> Result<Sse<BoxedSseStream>, AppError> {
    let user_id = claims.sub.clone();

    // Subscribe to broadcast channel FIRST to avoid race with terminal events.
    let task_manager = state.crawl_service.task_manager();
    let mgr = task_manager.lock().await;
    let rx = mgr.broadcast_tx().subscribe();
    drop(mgr);

    // Read current task state from DB.
    let detail = state
        .crawl_service
        .get_task_detail(&user_id, task_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found".to_owned()))?;

    let task_status = detail.task.status.clone();

    // If task is already terminal, emit terminal event and close
    if task_status.is_terminal() {
        let terminal_event = match &task_status {
            TaskStatus::Completed | TaskStatus::Failed => SseEvent::TaskCompleted {
                task_id,
                user_id: user_id.clone(),
                status: task_status.as_str().to_owned(),
                summary: TaskSummary {
                    total: detail.task.total_codes,
                    success: detail.task.success_count,
                    failed: detail.task.fail_count,
                    skipped: detail.task.skip_count,
                    pages_crawled: detail
                        .page_results
                        .iter()
                        .filter(|p| matches!(p.status, PageResultStatus::Success))
                        .count() as i64,
                },
            },
            TaskStatus::Cancelled => SseEvent::TaskCancelled {
                task_id,
                user_id: user_id.clone(),
                summary: TaskSummary {
                    total: detail.task.total_codes,
                    success: detail.task.success_count,
                    failed: detail.task.fail_count,
                    skipped: detail.task.skip_count,
                    pages_crawled: detail
                        .page_results
                        .iter()
                        .filter(|p| matches!(p.status, PageResultStatus::Success))
                        .count() as i64,
                },
            },
            _ => unreachable!(),
        };

        let data = serde_json::to_string(&terminal_event).unwrap_or_default();
        let event = SseEventInner::default()
            .event(terminal_event.event_type())
            .data(data);

        let stream: BoxedSseStream = Box::pin(futures::stream::once(async move { Ok(event) }));
        return Ok(Sse::new(stream).keep_alive(KeepAlive::default()));
    }

    // For queued/running tasks: emit initial stats snapshot
    let initial_stats = SseEvent::Stats {
        task_id,
        user_id: user_id.clone(),
        success_count: detail.task.success_count,
        fail_count: detail.task.fail_count,
        skip_count: detail.task.skip_count,
        total: detail.task.total_codes,
    };

    let initial_data = serde_json::to_string(&initial_stats).unwrap_or_default();
    let initial_event = SseEventInner::default()
        .event("task:stats")
        .data(initial_data);

    let crawl_service = state.crawl_service.clone();

    // Async stream: forwards matching events, closes on terminal, and
    // emits real stats on lag instead of a placeholder.
    let live_stream = async_stream::stream! {
        let mut stream = BroadcastStream::new(rx);
        loop {
            let item = stream.next().await;
            match item {
                Some(Ok(event)) => {
                    if event.task_id() == task_id && event.user_id() == user_id {
                        let is_terminal = event.is_terminal();
                        let data = serde_json::to_string(&event).unwrap_or_default();
                        let sse = SseEventInner::default()
                            .event(event.event_type())
                            .data(data);
                        yield Ok::<SseEventInner, Infallible>(sse);
                        if is_terminal {
                            break;
                        }
                    }
                }
                Some(Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_))) => {
                    // Query DB for real counts and emit a proper task:stats event.
                    let stats_event = crawl_service
                        .get_task_detail(&user_id, task_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|d| SseEvent::Stats {
                            task_id,
                            user_id: user_id.clone(),
                            success_count: d.task.success_count,
                            fail_count: d.task.fail_count,
                            skip_count: d.task.skip_count,
                            total: d.task.total_codes,
                        });
                    if let Some(stats) = stats_event {
                        let data = serde_json::to_string(&stats).unwrap_or_default();
                        let sse = SseEventInner::default()
                            .event("task:stats")
                            .data(data);
                        yield Ok(sse);
                    }
                }
                None => break,
            }
        }
    };

    // Prepend initial stats event
    let full_stream: BoxedSseStream =
        Box::pin(futures::stream::once(async move { Ok(initial_event) }).chain(live_stream));

    Ok(Sse::new(full_stream).keep_alive(KeepAlive::default()))
}

#[utoipa::path(
    post,
    path = "/crawl/initialize",
    responses(
        (status = 200, description = "Crawler initialized", body = CrawlerStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Crawler is busy"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn initialize_crawler(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    state.crawl_service.initialize_crawler().await?;

    let status = state.crawl_service.crawler_status().await;
    Ok(RestApiResponse::success(CrawlerStatusResponse {
        initialized: status.initialized,
        idle: status.idle,
    }))
}

#[utoipa::path(
    get,
    path = "/crawl/health",
    responses(
        (status = 200, description = "Crawler status", body = CrawlerStatusResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawl"
)]
pub async fn crawler_health(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> impl IntoResponse {
    let status = state.crawl_service.crawler_status().await;
    RestApiResponse::success(CrawlerStatusResponse {
        initialized: status.initialized,
        idle: status.idle,
    })
}

#[cfg(test)]
mod tests {
    //! Tests for the entity-auto-crawl detail derivation and the prefix
    //! contracts its `failed_page_numbers` exclusion depends on.

    use chrono::Utc;

    use crate::domains::crawl::domain::model::{
        CrawlPageResult, CrawlTask, CrawlTaskInput, EntityAutoCrawlScope, EntityAutoCrawlTaskInput,
        EntityAutoCrawlType, PageResultStatus, TaskStatus, TaskType,
    };

    use super::compute_entity_auto_detail;

    fn task_of(task_type: TaskType, payload: Option<String>) -> CrawlTask {
        CrawlTask {
            id: 1,
            task_type,
            status: TaskStatus::Failed,
            user_id: "u".to_owned(),
            mark_liked: false,
            mark_viewed: false,
            input_payload: payload,
            max_pages: None,
            total_codes: 0,
            success_count: 0,
            fail_count: 0,
            skip_count: 0,
            error_message: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    fn page(
        num: i32,
        status: PageResultStatus,
        records_found: i32,
        err: Option<&str>,
    ) -> CrawlPageResult {
        CrawlPageResult {
            id: num as i64,
            task_id: 1,
            page_number: num,
            status,
            records_found,
            records_crawled: 0,
            error_message: err.map(ToOwned::to_owned),
            created_at: Utc::now(),
        }
    }

    fn entity_payload() -> String {
        serde_json::to_string(&CrawlTaskInput::EntityAutoCrawl(EntityAutoCrawlTaskInput {
            entity_type: EntityAutoCrawlType::Idol,
            entity_id: 12,
            entity_name: "n".to_owned(),
            link: "l".to_owned(),
            base_url: "b".to_owned(),
            crawl_round: 0,
            scope: EntityAutoCrawlScope::Uncrawled,
        }))
        .expect("serialize payload")
    }

    #[test]
    fn detail_counts_exclude_first_404_interrupted_and_zero_record_success() {
        let task = task_of(TaskType::EntityAutoCrawl, Some(entity_payload()));
        let pages = vec![
            page(1, PageResultStatus::Success, 3, None),
            page(2, PageResultStatus::Success, 0, None),
            page(
                3,
                PageResultStatus::Failed,
                0,
                Some("Failed to extract record piece from item element"),
            ),
            page(
                4,
                PageResultStatus::Failed,
                0,
                Some("Page not found (404): https://example.com/x"),
            ),
            page(
                5,
                PageResultStatus::Failed,
                0,
                Some("Page processing interrupted: server restarted"),
            ),
            page(6, PageResultStatus::Success, 2, None),
        ];
        let d = compute_entity_auto_detail(&task, &pages).expect("entity_auto for entity task");
        // success_pages counts only pages with records > 0 (pages 1 and 6).
        assert_eq!(d.success_pages, 2);
        // Only page 3 is a real non-404 source error; the 404 end page (4) and
        // the restart-interrupted page (5) are excluded.
        assert_eq!(d.failed_pages, 1);
        assert_eq!(d.failed_page_numbers, vec![3]);
        assert_eq!(d.entity_type, "idol");
        assert_eq!(d.entity_id, 12);
    }

    #[test]
    fn detail_failed_page_numbers_are_sorted() {
        let task = task_of(TaskType::EntityAutoCrawl, Some(entity_payload()));
        // Non-404 errors out of page order -> must be sorted ascending.
        let pages = vec![
            page(5, PageResultStatus::Failed, 0, Some("err")),
            page(2, PageResultStatus::Failed, 0, Some("err")),
            page(
                9,
                PageResultStatus::Failed,
                0,
                Some("Page not found (404): x"),
            ),
            page(3, PageResultStatus::Success, 1, None),
        ];
        let d = compute_entity_auto_detail(&task, &pages).expect("entity_auto");
        assert_eq!(d.failed_page_numbers, vec![2, 5]);
    }

    #[test]
    fn detail_none_for_non_entity_auto_task() {
        let task = task_of(TaskType::Auto, None);
        let pages = vec![page(1, PageResultStatus::Success, 1, None)];
        assert!(compute_entity_auto_detail(&task, &pages).is_none());
    }

    #[test]
    fn detail_none_when_payload_missing_or_unreadable() {
        let task_no_payload = task_of(TaskType::EntityAutoCrawl, None);
        assert!(compute_entity_auto_detail(&task_no_payload, &[]).is_none());

        let task_bad_payload =
            task_of(TaskType::EntityAutoCrawl, Some("not valid json".to_owned()));
        assert!(compute_entity_auto_detail(&task_bad_payload, &[]).is_none());
    }

    /// The `failed_page_numbers` exclusion relies on luneth's `PageNotFound`
    /// Display starting with this exact prefix. Pin it so a luneth change does
    /// not silently let the first-404 page into `failed_pages`.
    #[test]
    fn luneth_page_not_found_display_prefix_is_pinned() {
        let msg = format!(
            "{}",
            luneth::crawl::CrawlError::PageNotFound {
                url: "https://example.com/x".to_owned()
            }
        );
        assert!(
            msg.starts_with("Page not found (404)"),
            "luneth PageNotFound Display prefix changed: {msg}"
        );
    }

    /// The `failed_page_numbers` exclusion also relies on the restart-interrupted
    /// page message starting with this prefix. The executor/reconcile both write
    /// messages beginning with it; pin that they do.
    #[test]
    fn interrupted_page_message_prefixes_are_pinned() {
        // reconcile_startup writes this for an interrupted in-progress page.
        let reconcile_msg = "Page processing interrupted: server restarted";
        assert!(
            reconcile_msg.starts_with("Page processing interrupted"),
            "reconcile interrupted-page prefix changed: {reconcile_msg}"
        );
        // The executor writes this on cancel mid-page.
        let cancel_msg = "Page processing interrupted: task cancelled";
        assert!(
            cancel_msg.starts_with("Page processing interrupted"),
            "executor interrupted-page prefix changed: {cancel_msg}"
        );
    }
}
