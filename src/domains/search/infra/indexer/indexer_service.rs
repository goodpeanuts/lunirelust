//! Background indexer that drains database outbox events into the search engine.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use sea_orm::DatabaseConnection;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration, Instant};

use crate::common::config::Config;
use crate::domains::search::domain::repository::outbox_repo::OutboxRepository as _;
use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::infra::outbox_repo_impl::OutboxRepo;
use crate::domains::search::SearchEntityType;

use super::event_processor::{process_event_batch, BatchProcessOutcome};
use super::reconciliation::{backfill_missing_vectors, reconcile_index};
use super::{full_sync, index_state};

const POLL_INTERVAL_SECS: u64 = 5;
const HEALTH_INTERVAL_SECS: u64 = 30;
const CLAIM_BATCH_SIZE: i64 = 50;
const LEASE_TIMEOUT_SECS: i64 = 300;
const RECONCILIATION_INTERVAL_SECS: u64 = 3600;

/// Log and ignore a best-effort operation result.
pub(super) fn ignore_result<T, E: std::fmt::Display>(result: Result<T, E>, label: &str) {
    if let Err(error) = result {
        tracing::debug!("{}: {}", label, error);
    }
}

/// Wrap raw embedding vectors in the search engine's embedder-keyed format.
pub(super) fn wrap_vectors(vectors: Option<Vec<f32>>) -> Option<serde_json::Value> {
    vectors.map(|vector| {
        serde_json::json!({
            "default": vector
        })
    })
}

pub struct IndexerService {
    db: DatabaseConnection,
    config: Config,
    search_repo: Arc<MeiliSearchRepo>,
    embedding_service: Arc<EmbeddingService>,
    meili_ready: Arc<AtomicBool>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl IndexerService {
    pub fn new(
        db: DatabaseConnection,
        config: Config,
        search_repo: Arc<MeiliSearchRepo>,
        embedding_service: Arc<EmbeddingService>,
        meili_ready: Arc<AtomicBool>,
    ) -> Self {
        Self {
            db,
            config,
            search_repo,
            embedding_service,
            meili_ready,
            task: Mutex::new(None),
        }
    }

    pub fn trigger_startup_sync(&self) {
        let mut task = self.task.lock().expect("indexer task lock");
        if task.is_some() {
            return;
        }
        let db = self.db.clone();
        let config = self.config.clone();
        let search_repo = self.search_repo.clone();
        let embedding_service = self.embedding_service.clone();
        let meili_ready = self.meili_ready.clone();

        *task = Some(tokio::spawn(async move {
            run_startup_sync(&db, &config, &search_repo, &embedding_service, &meili_ready).await;
            run_indexer_loop(&db, &config, &search_repo, &embedding_service, &meili_ready).await;
        }));
    }
}

impl Drop for IndexerService {
    fn drop(&mut self) {
        if let Some(task) = self.task.get_mut().expect("indexer task lock").take() {
            task.abort();
        }
    }
}

async fn check_embedding_health(
    search_repo: &MeiliSearchRepo,
    embedding_service: &EmbeddingService,
) {
    if search_repo.is_hybrid_ready() {
        embedding_service.check_health().await;
    } else {
        embedding_service.set_available(false);
    }
}

/// Startup order is deliberately strict:
/// initialize -> refresh invalidated content -> drain outbox -> repair IDs -> ready.
async fn run_startup_sync(
    db: &DatabaseConnection,
    _config: &Config,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
    meili_ready: &Arc<AtomicBool>,
) {
    let startup_start = std::time::Instant::now();
    meili_ready.store(false, Ordering::Relaxed);
    tracing::info!("Starting search index startup sync");

    if !search_repo.health_check().await {
        tracing::warn!("MeiliSearch is not available. Search will use SQL fallback.");
        return;
    }
    if let Err(error) = search_repo.init_index().await {
        tracing::error!("Failed to initialize MeiliSearch index: {error}");
        return;
    }

    check_embedding_health(search_repo, embedding_service).await;
    if !prepare_index(db, search_repo, embedding_service).await {
        tracing::warn!("Search index startup repair failed; keeping SQL fallback active");
        return;
    }

    meili_ready.store(true, Ordering::Relaxed);
    tracing::info!(
        elapsed_ms = startup_start.elapsed().as_millis() as u64,
        "Search index startup sync complete. MeiliSearch ready."
    );
}

/// Make a healthy index exact before exposing it to search traffic.
async fn prepare_index(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> bool {
    let state = match index_state::read(db).await {
        Ok(state) => state,
        Err(error) => {
            tracing::error!("Failed to read search index state: {error}");
            return false;
        }
    };
    if state.needs_rebuild {
        tracing::info!(
            "PostgreSQL mode invalidated the index; refreshing current database content"
        );
        if let Err(error) = index_state::retire_visible_backlog(db).await {
            tracing::error!("Failed to retire pre-rebuild events: {error}");
            return false;
        }
        if let Err(error) = full_sync::run_full_sync(db, search_repo, embedding_service).await {
            tracing::error!("Full refresh failed: {error}");
            return false;
        }
    } else {
        if !prune_stale_events(db).await
            || !process_pending_events(db, search_repo, embedding_service).await
        {
            return false;
        }
        let empty = match is_index_empty(search_repo).await {
            Ok(empty) => empty,
            Err(error) => {
                tracing::error!("Failed to inspect MeiliSearch index: {error}");
                return false;
            }
        };
        if empty {
            tracing::info!("Empty index detected; running batched full sync");
            if let Err(error) = full_sync::run_full_sync(db, search_repo, embedding_service).await {
                tracing::error!("Full sync failed: {error}");
                return false;
            }
        }
    }

    if !reconcile_index(db, search_repo, embedding_service).await {
        return false;
    }

    // Writes may have arrived while reconciliation was running. Drain once
    // more, then re-verify before declaring the index ready.
    let pending = OutboxRepo::count_pending(db).await.unwrap_or(-1);
    if pending > 0 {
        tracing::info!(
            pending,
            "Events arrived during repair; draining before final verification"
        );
        if !prune_stale_events(db).await
            || !process_pending_events(db, search_repo, embedding_service).await
            || !reconcile_index(db, search_repo, embedding_service).await
        {
            return false;
        }
    }

    if !matches!(OutboxRepo::count_pending(db).await, Ok(0)) {
        return false;
    }
    match index_state::acknowledge(db, state.generation).await {
        Ok(acknowledged) => acknowledged,
        Err(error) => {
            tracing::error!("Failed to acknowledge repaired search index: {error}");
            false
        }
    }
}

async fn is_index_empty(
    search_repo: &Arc<MeiliSearchRepo>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    for &entity_type in SearchEntityType::ALL {
        if search_repo.get_document_count(entity_type).await? > 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn prune_stale_events(db: &DatabaseConnection) -> bool {
    match OutboxRepo::mark_stale_upserts_processed(db).await {
        Ok(pruned) => {
            if pruned > 0 {
                tracing::info!(pruned, "Marked stale outbox upserts as processed");
            }
            true
        }
        Err(error) => {
            tracing::error!("Failed to prune stale outbox events: {error}");
            false
        }
    }
}

/// Drain the current backlog. A failed batch stops the tight startup loop so a
/// poison event cannot spin indefinitely while readiness remains false.
async fn process_pending_events(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) -> bool {
    loop {
        let events = match OutboxRepo::claim_pending(
            db,
            "startup-worker",
            CLAIM_BATCH_SIZE,
            LEASE_TIMEOUT_SECS,
        )
        .await
        {
            Ok(events) => events,
            Err(error) => {
                tracing::error!("Failed to claim startup events: {error}");
                return false;
            }
        };
        if events.is_empty() {
            return true;
        }

        let start = std::time::Instant::now();
        let outcome = process_event_batch(db, &events, search_repo, embedding_service).await;
        let ok = finish_claimed_batch(db, &outcome).await;
        tracing::info!(
            claimed = events.len(),
            processed = outcome.processed_ids.len(),
            failed = outcome.failures.len(),
            elapsed_ms = start.elapsed().as_millis() as u64,
            "Startup outbox batch processed"
        );
        if !ok {
            return false;
        }
    }
}

async fn finish_claimed_batch(db: &DatabaseConnection, outcome: &BatchProcessOutcome) -> bool {
    if let Err(error) = OutboxRepo::mark_processed_batch(db, &outcome.processed_ids).await {
        tracing::error!("Failed to mark outbox batch processed: {error}");
        for event_id in outcome
            .processed_ids
            .iter()
            .chain(outcome.failures.iter().map(|(event_id, _)| event_id))
        {
            ignore_result(
                OutboxRepo::release_claim(db, *event_id).await,
                "release_claim_after_mark_failure",
            );
        }
        return false;
    }

    for (event_id, message) in &outcome.failures {
        tracing::error!(event_id, "Failed to process outbox event: {message}");
        ignore_result(
            OutboxRepo::release_claim(db, *event_id).await,
            "release_claim",
        );
    }
    outcome.failures.is_empty()
}

#[expect(clippy::infinite_loop)]
async fn run_indexer_loop(
    db: &DatabaseConnection,
    _config: &Config,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
    meili_ready: &Arc<AtomicBool>,
) {
    let mut next_health_check = Instant::now() + Duration::from_secs(HEALTH_INTERVAL_SECS);
    let mut next_reconciliation =
        Instant::now() + Duration::from_secs(RECONCILIATION_INTERVAL_SECS);
    let mut index_prepared = meili_ready.load(Ordering::Relaxed);
    let mut embedding_was_available = embedding_service.is_available();
    let mut startup_backfill_done = false;

    loop {
        let loop_start = std::time::Instant::now();
        if !index_prepared || Instant::now() >= next_health_check {
            next_health_check = Instant::now() + Duration::from_secs(HEALTH_INTERVAL_SECS);
            if !search_repo.health_check().await {
                if meili_ready.swap(false, Ordering::Relaxed) {
                    tracing::warn!("MeiliSearch became unavailable");
                }
                index_prepared = false;
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }
            check_embedding_health(search_repo, embedding_service).await;
        }
        if !index_prepared {
            meili_ready.store(false, Ordering::Relaxed);
            if let Err(error) = search_repo.init_index().await {
                tracing::error!("Failed to initialize MeiliSearch index: {error}");
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }
            check_embedding_health(search_repo, embedding_service).await;
            if !prepare_index(db, search_repo, embedding_service).await {
                sleep(Duration::from_secs(HEALTH_INTERVAL_SECS)).await;
                continue;
            }
            index_prepared = true;
            meili_ready.store(true, Ordering::Relaxed);
            tracing::info!("MeiliSearch recovery complete");
        }

        let embedding_now_available = embedding_service.is_available();
        if embedding_now_available && (!embedding_was_available || !startup_backfill_done) {
            backfill_missing_vectors(search_repo, embedding_service).await;
            startup_backfill_done = true;
        }
        embedding_was_available = embedding_now_available;

        // claim_pending already reclaims expired leases atomically with its claim.
        let had_work = match OutboxRepo::claim_pending(
            db,
            "indexer-worker",
            CLAIM_BATCH_SIZE,
            LEASE_TIMEOUT_SECS,
        )
        .await
        {
            Ok(events) if !events.is_empty() => {
                let outcome =
                    process_event_batch(db, &events, search_repo, embedding_service).await;
                if !finish_claimed_batch(db, &outcome).await {
                    meili_ready.store(false, Ordering::Relaxed);
                    index_prepared = false;
                }
                tracing::info!(
                    claimed = events.len(),
                    processed = outcome.processed_ids.len(),
                    failed = outcome.failures.len(),
                    elapsed_ms = loop_start.elapsed().as_millis() as u64,
                    "Indexer batch processed"
                );
                true
            }
            Ok(_) => false,
            Err(error) => {
                tracing::error!("Failed to claim events: {error}");
                meili_ready.store(false, Ordering::Relaxed);
                index_prepared = false;
                false
            }
        };

        if Instant::now() >= next_reconciliation {
            next_reconciliation =
                Instant::now() + Duration::from_secs(RECONCILIATION_INTERVAL_SECS);
            let pending = OutboxRepo::count_pending(db).await.unwrap_or(-1);
            if pending == 0 {
                tracing::info!("Running periodic search reconciliation");
                if !prune_stale_events(db).await
                    || !reconcile_index(db, search_repo, embedding_service).await
                {
                    meili_ready.store(false, Ordering::Relaxed);
                    index_prepared = false;
                } else if embedding_service.is_available() {
                    backfill_missing_vectors(search_repo, embedding_service).await;
                }
            }
        }

        // Drain a backlog promptly, but keep idle database polling inexpensive.
        sleep(if had_work {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(POLL_INTERVAL_SECS)
        })
        .await;
    }
}
