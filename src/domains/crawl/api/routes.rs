use axum::routing::{get, post};
use axum::Router;

use crate::common::app_state::AppState;
use crate::common::openapi::SecurityAddon;
use crate::domains::crawl::dto::task_dto::{
    CodeResultResponse, CrawlerStatusResponse, EntityAutoCrawlDetail, EntityAutoCrawlTaskItem,
    EntityAutoCrawlTaskResponse, EntityProgressItem, EntityProgressListResponse,
    EntityProgressSummary, PageResultResponse, StartAutoRequest, StartBatchRequest,
    StartEntityAutoCrawlRequest, StartIdolRequest, StartUpdateRequest, TaskDetailResponse,
    TaskListItem, TaskListResponse, TaskResponse,
};

use super::handlers::{
    __path_cancel_task, __path_crawler_health, __path_get_entity_progress_summary,
    __path_get_task_detail, __path_initialize_crawler, __path_list_entity_progress,
    __path_list_tasks, __path_start_auto, __path_start_batch, __path_start_entity_auto_crawl,
    __path_start_idol, __path_start_update, __path_stream_task, cancel_task, crawler_health,
    get_entity_progress_summary, get_task_detail, initialize_crawler, list_entity_progress,
    list_tasks, start_auto, start_batch, start_entity_auto_crawl, start_idol, start_update,
    stream_task,
};

pub fn crawl_routes() -> Router<AppState> {
    Router::new()
        .route("/batch", post(start_batch))
        .route("/auto", post(start_auto))
        .route("/update", post(start_update))
        .route("/idol", post(start_idol))
        .route("/entity-auto-crawl", post(start_entity_auto_crawl))
        .route("/entity-progress", get(list_entity_progress))
        .route("/entity-progress/summary", get(get_entity_progress_summary))
        .route("/initialize", post(initialize_crawler))
        .route("/health", get(crawler_health))
        .route("/tasks", get(list_tasks))
        .route("/tasks/{id}", get(get_task_detail))
        .route("/tasks/{id}/cancel", post(cancel_task))
        .route("/tasks/{id}/stream", get(stream_task))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        start_batch, start_auto, start_update, start_idol, start_entity_auto_crawl,
        list_entity_progress, get_entity_progress_summary, initialize_crawler, crawler_health,
        list_tasks, get_task_detail, cancel_task, stream_task
    ),
    components(schemas(
        StartBatchRequest, StartAutoRequest, StartUpdateRequest, StartIdolRequest,
        StartEntityAutoCrawlRequest, TaskResponse, TaskListItem, TaskListResponse,
        TaskDetailResponse, CodeResultResponse, PageResultResponse, CrawlerStatusResponse,
        EntityAutoCrawlTaskItem, EntityAutoCrawlTaskResponse, EntityProgressItem,
        EntityProgressListResponse, EntityProgressSummary, EntityAutoCrawlDetail
    )),
    tags((name = "Crawl", description = "Crawl task management and progress endpoints")),
    security(("bearer_auth" = [])),
    modifiers(&SecurityAddon)
)]
pub struct CrawlApiDoc;
