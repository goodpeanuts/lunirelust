//! `IndexerService`: background task that consumes outbox events and syncs to `MeiliSearch`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::time::{sleep, Duration};

use crate::common::config::Config;
use crate::domains::search::domain::repository::outbox_repo::OutboxRepository as _;
use crate::domains::search::domain::repository::search_repo::SearchRepository as _;
use crate::domains::search::infra::embedding::embedding_service::EmbeddingService;
use crate::domains::search::infra::meilisearch::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::infra::outbox_repo_impl::OutboxRepo;
use crate::domains::search::SearchEntityType;

use super::event_processor::process_event;
use super::full_sync;
use super::reconciliation::{backfill_missing_vectors, reconcile_counts};

/// Interval between outbox polling cycles.
const POLL_INTERVAL_SECS: u64 = 1;
/// Maximum number of events claimed per polling cycle.
const CLAIM_BATCH_SIZE: i64 = 50;
/// Duration after which a claimed event lease expires and can be reclaimed.
const LEASE_TIMEOUT_SECS: i64 = 300; // 5 minutes
/// Interval between periodic PostgreSQL-vs-MeiliSearch reconciliation checks.
const RECONCILIATION_INTERVAL_SECS: u64 = 3600; // 1 hour

/// Log and ignore a best-effort operation result.
pub(super) fn ignore_result<T, E: std::fmt::Display>(result: Result<T, E>, label: &str) {
    if let Err(e) = result {
        tracing::debug!("{}: {}", label, e);
    }
}

/// Wrap raw embedding vectors into `MeiliSearch`'s embedder-keyed format:
/// `{"default": [0.1, 0.2, ...]}`.
pub(super) fn wrap_vectors(vectors: Option<Vec<f32>>) -> Option<serde_json::Value> {
    vectors.map(|v| {
        serde_json::json!({
            "default": v
        })
    })
}

/// Background indexer service.
///
/// On startup, initializes the `MeiliSearch` index, runs a full sync if the
/// index is empty, then enters a polling loop that consumes outbox events.
/// Periodically reconciles `PostgreSQL` and `MeiliSearch` document counts.
pub struct IndexerService {
    /// `PostgreSQL` connection for event processing and full sync.
    db: DatabaseConnection,
    /// Application configuration.
    config: Config,
    /// `MeiliSearch` repository for document operations.
    search_repo: Arc<MeiliSearchRepo>,
    /// Embedding service for vector generation during indexing.
    embedding_service: Arc<EmbeddingService>,
    /// Shared readiness flag — set to `true` once the index is populated and backlog is drained.
    meili_ready: Arc<AtomicBool>,
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
        }
    }

    pub fn trigger_startup_sync(&self) {
        let db = self.db.clone();
        let config = self.config.clone();
        let search_repo = self.search_repo.clone();
        let embedding_service = self.embedding_service.clone();
        let meili_ready = self.meili_ready.clone();

        tokio::spawn(async move {
            run_startup_sync(&db, &config, &search_repo, &embedding_service, &meili_ready).await;
            run_indexer_loop(&db, &config, &search_repo, &embedding_service, &meili_ready).await;
        });
    }
}

/// Run the one-time startup sync sequence:
/// 1. Check `MeiliSearch` health
/// 2. Initialize index settings
/// 3. Probe embedding service
/// 4. Full sync if index is empty, otherwise reconcile counts
/// 5. Drain pending outbox events
async fn run_startup_sync(
    db: &DatabaseConnection,
    _config: &Config,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
    meili_ready: &Arc<AtomicBool>,
) {
    tracing::info!("Starting search index startup sync...");
    let startup_start = std::time::Instant::now();

    if !search_repo.health_check().await {
        tracing::warn!("MeiliSearch is not available. Search will use SQL fallback.");
        return;
    }
    tracing::info!(
        elapsed_ms = startup_start.elapsed().as_millis() as u64,
        "Startup: MeiliSearch health check passed"
    );

    if let Err(e) = search_repo.init_index().await {
        tracing::error!("Failed to initialize MeiliSearch index: {}", e);
        return;
    }
    tracing::info!(
        elapsed_ms = startup_start.elapsed().as_millis() as u64,
        "Startup: index initialized"
    );

    // Probe embedding service health before full sync so that initial
    // documents can include vectors when vLLM is reachable.
    embedding_service.check_health().await;
    tracing::info!(
        elapsed_ms = startup_start.elapsed().as_millis() as u64,
        embedding_available = embedding_service.is_available(),
        "Startup: embedding service probed"
    );

    let doc_count = search_repo
        .get_document_count(SearchEntityType::Record)
        .await
        .unwrap_or(0);
    if doc_count == 0 {
        tracing::info!("Empty index detected, running full sync...");
        if let Err(e) = full_sync::run_full_sync(db, search_repo, embedding_service).await {
            tracing::error!("Full sync failed: {}", e);
            return;
        }
        tracing::info!(
            elapsed_ms = startup_start.elapsed().as_millis() as u64,
            "Startup: full sync completed"
        );
    } else {
        // Non-empty index — verify completeness by comparing PostgreSQL counts.
        // If any entity type is missing documents, run full sync to repair.
        let reconcile_ok = reconcile_counts(db, search_repo, embedding_service).await;
        if !reconcile_ok {
            tracing::warn!("Reconciliation failed, keeping SQL fallback active");
            return;
        }
        tracing::info!(
            elapsed_ms = startup_start.elapsed().as_millis() as u64,
            "Startup: reconciliation check passed"
        );
    }

    // Process pending outbox events
    let pending = OutboxRepo::count_pending(db).await.unwrap_or(0);
    if pending > 0 {
        tracing::info!(pending, "Startup: draining pending outbox events");
        meili_ready.store(false, Ordering::Relaxed);
        process_pending_events(db, search_repo, embedding_service).await;
    }

    let remaining = OutboxRepo::count_pending(db).await.unwrap_or(0);
    if remaining == 0 {
        tracing::info!(
            elapsed_ms = startup_start.elapsed().as_millis() as u64,
            remaining_pending = remaining,
            "Search index startup sync complete. MeiliSearch ready."
        );
        meili_ready.store(true, Ordering::Relaxed);
    }
}

/// Claim and process pending outbox events in a tight loop until none remain.
/// Used during startup to drain the backlog before enabling `MeiliSearch`.
async fn process_pending_events(
    db: &DatabaseConnection,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
) {
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
            Err(e) => {
                tracing::error!("Failed to claim events: {}", e);
                break;
            }
        };
        if events.is_empty() {
            break;
        }
        for event in &events {
            if let Err(e) = process_event(db, event, search_repo, embedding_service).await {
                tracing::error!("Failed to process event {}: {}", event.id, e);
                ignore_result(
                    OutboxRepo::release_claim(db, event.id).await,
                    "release_claim",
                );
                continue;
            }
            ignore_result(
                OutboxRepo::mark_processed(db, event.id).await,
                "mark_processed",
            );
        }
    }
}

/// Main indexer loop: polls for outbox events, handles `MeiliSearch` recovery,
/// embedding service recovery, and periodic reconciliation.
#[expect(clippy::infinite_loop)]
#[expect(clippy::too_many_lines)]
async fn run_indexer_loop(
    db: &DatabaseConnection,
    _config: &Config,
    search_repo: &Arc<MeiliSearchRepo>,
    embedding_service: &Arc<EmbeddingService>,
    meili_ready: &Arc<AtomicBool>,
) {
    let mut reconciliation_timer = 0u64;
    // If the startup sync already completed successfully, meili_ready is true
    // and we don't need to re-run full sync. Only re-run if Meili was down
    // during startup (meili_ready=false) or went down and came back.
    let mut full_sync_done = meili_ready.load(Ordering::Relaxed);
    let mut embedding_was_available = embedding_service.is_available();
    // On first loop iteration, backfill vectors if embedding is available but
    // the index was populated by a prior run (possibly without vLLM). This
    // covers the common restart case where vLLM is healthy from the start.
    let mut startup_backfill_done = false;

    loop {
        let loop_start = std::time::Instant::now();

        if !search_repo.health_check().await {
            if meili_ready.load(Ordering::Relaxed) {
                tracing::warn!("MeiliSearch became unavailable");
                meili_ready.store(false, Ordering::Relaxed);
            }
            full_sync_done = false;
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Initialize and repopulate the index if it hasn't been done yet
        // (covers the case where Meili was unavailable during startup)
        if !full_sync_done {
            if let Err(e) = search_repo.init_index().await {
                tracing::error!("Failed to initialize MeiliSearch index: {}", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }
            tracing::info!("Running full sync in indexer loop...");
            if let Err(e) = full_sync::run_full_sync(db, search_repo, embedding_service).await {
                tracing::error!("Full sync failed: {}", e);
                // Do NOT set full_sync_done — retry on next iteration.
                // The index may be partially written, but run_full_sync is
                // idempotent (it overwrites all documents and tombstones).
                sleep(Duration::from_secs(30)).await;
                continue;
            }
            full_sync_done = true;
        }

        embedding_service.check_health().await;

        // Detect vLLM recovery → backfill documents missing vectors
        let embedding_now_available = embedding_service.is_available();
        if embedding_now_available && !embedding_was_available {
            tracing::info!("vLLM embedding service recovered, starting vector backfill...");
            backfill_missing_vectors(search_repo, embedding_service).await;
        }
        // One-time startup backfill: if embedding is healthy and the index
        // already existed, backfill any documents that were indexed without
        // vectors during a prior run when vLLM was unavailable.
        if !startup_backfill_done && full_sync_done && embedding_now_available {
            backfill_missing_vectors(search_repo, embedding_service).await;
            startup_backfill_done = true;
        }
        embedding_was_available = embedding_now_available;
        ignore_result(
            OutboxRepo::reclaim_expired_claims(db, LEASE_TIMEOUT_SECS).await,
            "reclaim_expired_claims",
        );

        match OutboxRepo::claim_pending(db, "indexer-worker", CLAIM_BATCH_SIZE, LEASE_TIMEOUT_SECS)
            .await
        {
            Ok(events) if !events.is_empty() => {
                let batch_size = events.len();
                for event in &events {
                    match process_event(db, event, search_repo, embedding_service).await {
                        Ok(()) => {
                            ignore_result(
                                OutboxRepo::mark_processed(db, event.id).await,
                                "mark_processed",
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                event_id = event.id,
                                entity_type = %event.entity_type,
                                entity_id = %event.entity_id,
                                "Failed to process event: {e}"
                            );
                            ignore_result(
                                OutboxRepo::release_claim(db, event.id).await,
                                "release_claim",
                            );
                        }
                    }
                }
                let pending = OutboxRepo::count_pending(db).await.unwrap_or(-1);
                tracing::info!(
                    processed = batch_size,
                    pending,
                    elapsed_ms = loop_start.elapsed().as_millis() as u64,
                    "Indexer batch processed"
                );
                if !meili_ready.load(Ordering::Relaxed) && pending == 0 {
                    tracing::info!("Outbox backlog drained. MeiliSearch ready.");
                    meili_ready.store(true, Ordering::Relaxed);
                }
            }
            Ok(_) => {
                if !meili_ready.load(Ordering::Relaxed) {
                    let pending = OutboxRepo::count_pending(db).await.unwrap_or(0);
                    if pending == 0 {
                        meili_ready.store(true, Ordering::Relaxed);
                    }
                }
            }
            Err(e) => tracing::error!("Failed to claim events: {}", e),
        }

        reconciliation_timer += POLL_INTERVAL_SECS;
        if reconciliation_timer >= RECONCILIATION_INTERVAL_SECS {
            reconciliation_timer = 0;
            tracing::info!("Running periodic reconciliation...");
            // Compare PostgreSQL vs MeiliSearch counts to detect data loss
            reconcile_counts(db, search_repo, embedding_service).await;
            // Periodically backfill missing vectors when embedding is available
            if embedding_service.is_available() {
                backfill_missing_vectors(search_repo, embedding_service).await;
            }
        }

        sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}
