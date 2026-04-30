use axum::routing::{get, post};
use axum::Router;

use crate::common::app_state::AppState;
use crate::common::openapi::SecurityAddon;
use crate::domains::crawl::dto::task_dto::{
    CodeResultResponse, PageResultResponse, StartAutoRequest, StartBatchRequest,
    StartUpdateRequest, TaskDetailResponse, TaskListItem, TaskListResponse, TaskResponse,
};

use super::handlers::{
    __path_cancel_task, __path_get_task_detail, __path_list_tasks, __path_start_auto,
    __path_start_batch, __path_start_update, __path_stream_task, cancel_task, get_task_detail,
    list_tasks, start_auto, start_batch, start_update, stream_task,
};

pub fn crawl_routes() -> Router<AppState> {
    Router::new()
        .route("/batch", post(start_batch))
        .route("/auto", post(start_auto))
        .route("/update", post(start_update))
        .route("/tasks", get(list_tasks))
        .route("/tasks/{id}", get(get_task_detail))
        .route("/tasks/{id}/cancel", post(cancel_task))
        .route("/tasks/{id}/stream", get(stream_task))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(start_batch, start_auto, start_update, list_tasks, get_task_detail, cancel_task, stream_task),
    components(schemas(
        StartBatchRequest, StartAutoRequest, StartUpdateRequest,
        TaskResponse, TaskListItem, TaskListResponse, TaskDetailResponse,
        CodeResultResponse, PageResultResponse
    )),
    tags((name = "Crawl", description = "Crawl task management and progress endpoints")),
    security(("bearer_auth" = [])),
    modifiers(&SecurityAddon)
)]
pub struct CrawlApiDoc;
