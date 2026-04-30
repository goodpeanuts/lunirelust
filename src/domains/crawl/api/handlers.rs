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
use crate::domains::crawl::domain::model::{PageResultStatus, TaskStatus, TaskType};
use crate::domains::crawl::dto::task_dto::{
    CodeResultResponse, ListTasksQuery, PageResultResponse, SseEvent, StartAutoRequest,
    StartBatchRequest, StartUpdateRequest, TaskDetailResponse, TaskListItem, TaskListResponse,
    TaskResponse, TaskSummary,
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
        .start_batch(&claims.sub, req.codes, req.mark_liked, req.mark_viewed)
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
        .start_update(&claims.sub, req.liked_only, req.created_after)
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
            }))
        }
        None => Err(AppError::NotFound("Task not found".to_owned())),
    }
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
