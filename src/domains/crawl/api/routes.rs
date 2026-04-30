use axum::routing::{get, post};
use axum::Router;

use crate::common::app_state::AppState;

use super::handlers::{
    cancel_task, get_task_detail, list_tasks, start_auto, start_batch, start_update, stream_task,
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
#[openapi()]
pub struct CrawlApiDoc;
