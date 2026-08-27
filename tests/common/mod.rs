use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::{json, Value as JsonValue};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use lunirelust::domains::search::constants::{FILTERABLE_ATTRIBUTES, SEARCHABLE_ATTRIBUTES};

// Fixed task UID used by the fake settings PATCH success path so tests can
// deterministically drive a later task-poll failure.
const EMBEDDER_TASK_UID: u32 = 42;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeEmbedderFailureMode {
    PatchRejected,
    TaskFailed,
}

#[derive(Clone)]
struct FakeMeiliState {
    embedder_failure_mode: FakeEmbedderFailureMode,
    next_task_uid: Arc<AtomicU32>,
    embedder_patch_requests: Arc<AtomicUsize>,
    keyword_search_requests: Arc<AtomicUsize>,
    documents: Arc<Mutex<HashMap<String, JsonValue>>>,
}

pub struct FakeMeiliServer {
    pub base_url: String,
    embedder_patch_requests: Arc<AtomicUsize>,
    keyword_search_requests: Arc<AtomicUsize>,
    handle: JoinHandle<()>,
}

impl FakeMeiliServer {
    pub fn embedder_patch_requests(&self) -> usize {
        self.embedder_patch_requests.load(Ordering::Relaxed)
    }

    pub fn keyword_search_requests(&self) -> usize {
        self.keyword_search_requests.load(Ordering::Relaxed)
    }
}

impl Drop for FakeMeiliServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub async fn spawn_fake_meili_server(
    embedder_failure_mode: FakeEmbedderFailureMode,
) -> FakeMeiliServer {
    let state = FakeMeiliState {
        embedder_failure_mode,
        next_task_uid: Arc::new(AtomicU32::new(100)),
        embedder_patch_requests: Arc::new(AtomicUsize::new(0)),
        keyword_search_requests: Arc::new(AtomicUsize::new(0)),
        documents: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(get_health))
        .route("/indexes/{index_uid}", get(get_index))
        .route(
            "/indexes/{index_uid}/settings",
            get(get_settings).patch(patch_settings),
        )
        .route("/indexes/{index_uid}/stats", get(get_stats))
        .route(
            "/indexes/{index_uid}/documents/fetch",
            post(post_documents_fetch),
        )
        .route(
            "/indexes/{index_uid}/documents/delete-batch",
            post(delete_documents),
        )
        .route(
            "/indexes/{index_uid}/documents",
            post(post_documents).put(post_documents),
        )
        .route(
            "/indexes/{index_uid}/documents/{doc_id}",
            delete(delete_document),
        )
        .route(
            "/indexes/{index_uid}/search",
            get(search_documents).post(search_documents),
        )
        .route("/tasks/{task_uid}", get(get_task))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("binding fake meilisearch test server");
    let address = listener
        .local_addr()
        .expect("reading fake meilisearch test server address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serving fake meilisearch test server");
    });

    FakeMeiliServer {
        base_url: format!("http://{address}"),
        embedder_patch_requests: state.embedder_patch_requests,
        keyword_search_requests: state.keyword_search_requests,
        handle,
    }
}

async fn get_health() -> Json<JsonValue> {
    Json(json!({ "status": "available" }))
}

async fn get_index(Path(index_uid): Path<String>) -> Json<JsonValue> {
    Json(json!({
        "uid": index_uid,
        "primaryKey": "id",
        "createdAt": "2026-04-19T00:00:00Z",
        "updatedAt": "2026-04-19T00:00:00Z"
    }))
}

async fn get_settings() -> Json<JsonValue> {
    // Return the production search/filter settings but no embedders, so
    // init_index() always attempts the embedder configuration step.
    Json(json!({
        "searchableAttributes": SEARCHABLE_ATTRIBUTES,
        "filterableAttributes": FILTERABLE_ATTRIBUTES,
        "embedders": {}
    }))
}

async fn patch_settings(State(state): State<FakeMeiliState>) -> (StatusCode, Json<JsonValue>) {
    state
        .embedder_patch_requests
        .fetch_add(1, Ordering::Relaxed);

    match state.embedder_failure_mode {
        FakeEmbedderFailureMode::PatchRejected => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": "invalid embedder configuration"
            })),
        ),
        FakeEmbedderFailureMode::TaskFailed => (
            StatusCode::ACCEPTED,
            Json(json!({
                "taskUid": EMBEDDER_TASK_UID
            })),
        ),
    }
}

async fn get_stats(State(state): State<FakeMeiliState>) -> Json<JsonValue> {
    let document_count = state.documents.lock().await.len();
    Json(json!({
        "numberOfDocuments": document_count,
        "numberOfEmbeddedDocuments": 0,
        "numberOfEmbeddings": 0,
        "rawDocumentDbSize": 0,
        "avgDocumentSize": 0,
        "isIndexing": false,
        "fieldDistribution": {}
    }))
}

async fn post_documents_fetch(
    State(state): State<FakeMeiliState>,
    Json(body): Json<JsonValue>,
) -> Json<JsonValue> {
    let requested_entity_type = body
        .get("filter")
        .and_then(JsonValue::as_str)
        .and_then(|filter| filter.split('"').nth(1));
    let offset = body
        .get("offset")
        .and_then(JsonValue::as_u64)
        .unwrap_or_default() as usize;
    let limit = body.get("limit").and_then(JsonValue::as_u64).unwrap_or(20) as usize;

    let documents = state.documents.lock().await;
    let mut matching: Vec<JsonValue> = documents
        .values()
        .filter(|document| {
            requested_entity_type.is_none_or(|entity_type| {
                document.get("entity_type").and_then(JsonValue::as_str) == Some(entity_type)
            })
        })
        .cloned()
        .collect();
    matching.sort_by(|left, right| {
        left.get("id")
            .and_then(JsonValue::as_str)
            .cmp(&right.get("id").and_then(JsonValue::as_str))
    });
    let total = matching.len();
    let results: Vec<JsonValue> = matching.into_iter().skip(offset).take(limit).collect();

    Json(json!({
        "results": results,
        "total": total
    }))
}

async fn post_documents(
    State(state): State<FakeMeiliState>,
    Json(body): Json<JsonValue>,
) -> (StatusCode, Json<JsonValue>) {
    if let Some(documents) = body.as_array() {
        let mut stored = state.documents.lock().await;
        for document in documents {
            if let Some(id) = document.get("id").and_then(JsonValue::as_str) {
                stored.insert(id.to_owned(), document.clone());
            }
        }
    }
    let task_uid = state.next_task_uid.fetch_add(1, Ordering::Relaxed);
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "enqueuedAt": "2026-04-19T00:00:00Z",
            "indexUid": "luna_search",
            "status": "enqueued",
            "type": "documentAdditionOrUpdate",
            "taskUid": task_uid
        })),
    )
}

async fn delete_document(
    State(state): State<FakeMeiliState>,
    Path(doc_id): Path<String>,
) -> (StatusCode, Json<JsonValue>) {
    state.documents.lock().await.remove(&doc_id);
    accepted_delete_task(&state)
}

async fn delete_documents(
    State(state): State<FakeMeiliState>,
    Json(doc_ids): Json<Vec<String>>,
) -> (StatusCode, Json<JsonValue>) {
    let mut documents = state.documents.lock().await;
    for doc_id in doc_ids {
        documents.remove(&doc_id);
    }
    drop(documents);
    accepted_delete_task(&state)
}

fn accepted_delete_task(state: &FakeMeiliState) -> (StatusCode, Json<JsonValue>) {
    let task_uid = state.next_task_uid.fetch_add(1, Ordering::Relaxed);
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "enqueuedAt": "2026-04-19T00:00:00Z",
            "indexUid": "luna_search",
            "status": "enqueued",
            "type": "documentDeletion",
            "taskUid": task_uid
        })),
    )
}

async fn search_documents(State(state): State<FakeMeiliState>) -> Json<JsonValue> {
    state
        .keyword_search_requests
        .fetch_add(1, Ordering::Relaxed);

    // Return one deterministic keyword hit so the regression test can prove
    // the backend stayed on Meili keyword search instead of SQL fallback.
    Json(json!({
        "hits": [
            {
                "id": "record__ABC-123",
                "title": "ABC-123 Search Result",
                "entity_type": "record",
                "entity_id": "ABC-123",
                "permission": 0,
                "_formatted": {
                    "title": "<em>ABC-123</em> Search Result"
                },
                "_rankingScore": 1.0
            }
        ],
        "query": "ABC-123",
        "processingTimeMs": 1,
        "limit": 20,
        "offset": 0,
        "estimatedTotalHits": 1
    }))
}

async fn get_task(
    State(state): State<FakeMeiliState>,
    Path(task_uid): Path<u32>,
) -> Json<JsonValue> {
    if task_uid == EMBEDDER_TASK_UID
        && state.embedder_failure_mode == FakeEmbedderFailureMode::TaskFailed
    {
        return Json(json!({
            "uid": task_uid,
            "status": "failed",
            "error": {
                "message": "embedder task failed"
            }
        }));
    }

    Json(json!({
        "uid": task_uid,
        "status": "succeeded"
    }))
}
