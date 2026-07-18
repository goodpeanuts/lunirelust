#![allow(clippy::unwrap_used)]
//! Integration tests for the entity-auto-crawl repository against the test
//! database: round derivation, Model D (claim never changes the round; only
//! successful completion advances it via `advance_round_on_complete`), summary
//! (remaining <= total), the per-type advisory lock preventing concurrent
//! double-claims, and cancellation leaving the round untouched.
//!
//! These tests share global tables (`crawl_task`, `crawl_entity_progress`,
//! `idol`), so each holds a transaction-scoped advisory lock on a dedicated
//! connection (key 91117, distinct from the claim lock's key) to serialize
//! across nextest's per-test processes.

use std::collections::HashSet;

use lunirelust::common::config::{setup_database, Config};
use lunirelust::domains::crawl::{CrawlRepo, EntityAutoCrawlType, EntityProgressRepository as _};
use sea_orm::{
    ConnectionTrait as _, DatabaseBackend, DatabaseConnection, DatabaseTransaction, Statement,
    TransactionTrait as _,
};

const TEST_LINK_PREFIX: &str = "https://enttest";
/// Serialization key for these integration tests. Distinct from the claim
/// advisory lock's key (`hashtext('entity_auto_crawl:...')`) so holding it does
/// not interfere with the claim flow under test.
const SERIAL_KEY: i64 = 91_117;

async fn db() -> DatabaseConnection {
    let _env = dotenvy::from_filename(".env.test");
    let config = Config::from_env().expect("config");
    setup_database(&config).await.expect("test db")
}

/// Hold a transaction-scoped advisory lock for the test's duration, serializing
/// these DB-shared tests across nextest processes. Returned transaction is held
/// (idle) and released on drop.
async fn begin_serial(db: &DatabaseConnection) -> DatabaseTransaction {
    let txn = db.begin().await.unwrap();
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1)",
        [SERIAL_KEY.into()],
    ))
    .await
    .unwrap();
    txn
}

/// Remove all entity-auto-crawl state so each test starts deterministically.
async fn clean_entity_auto_state(db: &DatabaseConnection) {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM crawl_task WHERE task_type = 'entity_auto_crawl'",
        [],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM crawl_entity_progress",
        [],
    ))
    .await
    .unwrap();
}

/// Insert `n` test idols with non-empty, distinctly-prefixed links (cleaned up
/// by `remove_test_idols`). Links are fake but the claim flow never crawls them.
async fn insert_test_idols(db: &DatabaseConnection, n: i64) {
    for i in 0..n {
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO idol (name, link, manual) VALUES ($1, $2, false)",
            [
                format!("enttest-{i}").into(),
                format!("{TEST_LINK_PREFIX}-{i}").into(),
            ],
        ))
        .await
        .unwrap();
    }
}

async fn remove_test_idols(db: &DatabaseConnection) {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM idol WHERE link LIKE $1",
        [format!("{TEST_LINK_PREFIX}-%").into()],
    ))
    .await
    .unwrap();
}

/// `(min, max)` of `last_crawled_round` over `crawl_entity_progress` rows.
async fn round_min_max(db: &DatabaseConnection) -> (Option<i64>, Option<i64>) {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT MIN(last_crawled_round)::bigint AS mn, MAX(last_crawled_round)::bigint AS mx \
             FROM crawl_entity_progress",
            [],
        ))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "mn").unwrap(),
        row.try_get("", "mx").unwrap(),
    )
}

async fn entity_round(db: &DatabaseConnection, entity_id: i64) -> Option<i64> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT last_crawled_round::bigint AS r FROM crawl_entity_progress \
             WHERE entity_type = 'idol' AND entity_id = $1",
            [entity_id.into()],
        ))
        .await
        .unwrap()?;
    Some(row.try_get::<i64>("", "r").unwrap())
}

#[tokio::test]
async fn current_round_is_zero_on_empty_progress() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean_entity_auto_state(&db).await;

    let repo = CrawlRepo;
    let cr = repo
        .current_round(&db, EntityAutoCrawlType::Idol)
        .await
        .unwrap();
    assert_eq!(cr, 0);
}

#[tokio::test]
async fn claim_does_not_change_round_and_summary_remaining_le_total() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean_entity_auto_state(&db).await;
    insert_test_idols(&db, 6).await;
    let repo = CrawlRepo;

    let before = repo
        .progress_summary(&db, EntityAutoCrawlType::Idol)
        .await
        .unwrap();
    // remaining is on the same link<>'' base as total, so it cannot exceed it.
    assert!(
        before.remaining <= before.total,
        "{} <= {}",
        before.remaining,
        before.total
    );

    let claimed = repo
        .claim_uncrawled(
            &db,
            EntityAutoCrawlType::Idol,
            3,
            0,
            "00000000-0000-0000-0000-000000000001",
            "https://x/",
        )
        .await
        .unwrap();
    assert_eq!(claimed.len(), 3, "should claim up to 3 of the 6 candidates");

    // Model D: claim does NOT advance the round. Each claimed entity's row is at
    // round 0 (inserted at 0, not bumped).
    for c in &claimed {
        assert_eq!(
            entity_round(&db, c.entity_id).await,
            Some(0),
            "claim must not change the round (Model D)"
        );
    }

    let after = repo
        .progress_summary(&db, EntityAutoCrawlType::Idol)
        .await
        .unwrap();
    assert!(after.remaining <= after.total);

    // Invariant max - min <= 1 holds (all rows still at 0).
    let (mn, mx) = round_min_max(&db).await;
    let (mn, mx) = (mn.unwrap_or(0), mx.unwrap_or(0));
    assert!(mx - mn <= 1, "round invariant violated: min={mn} max={mx}");

    remove_test_idols(&db).await;
}

/// Model D: only successful completion advances the round, to `crawl_round + 1`,
/// monotonically (GREATEST).
#[tokio::test]
async fn complete_advances_round_monotonically() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean_entity_auto_state(&db).await;
    insert_test_idols(&db, 2).await;
    let repo = CrawlRepo;
    let user = "00000000-0000-0000-0000-000000000001";

    let claimed = repo
        .claim_uncrawled(&db, EntityAutoCrawlType::Idol, 1, 0, user, "https://x/")
        .await
        .unwrap();
    let one = &claimed[0];
    // Claim leaves round at 0.
    assert_eq!(entity_round(&db, one.entity_id).await, Some(0));

    // Successful completion at crawl_round 0 advances to 1.
    repo.advance_round_on_complete(&db, EntityAutoCrawlType::Idol, one.entity_id, 0)
        .await
        .unwrap();
    assert_eq!(entity_round(&db, one.entity_id).await, Some(1));

    // Idempotent / monotonic: a stale completion for the same round does not
    // lower it (GREATEST).
    repo.advance_round_on_complete(&db, EntityAutoCrawlType::Idol, one.entity_id, 0)
        .await
        .unwrap();
    assert_eq!(
        entity_round(&db, one.entity_id).await,
        Some(1),
        "duplicate completion must not change the round"
    );

    remove_test_idols(&db).await;
}

/// Two concurrent same-type claims must not double-claim any entity: their
/// selected entity-id sets are disjoint (the per-type advisory lock serializes
/// them). Empirical guard for the I1/I2 fix.
#[tokio::test]
async fn concurrent_claims_do_not_double_claim() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean_entity_auto_state(&db).await;
    insert_test_idols(&db, 12).await;
    let repo = CrawlRepo;

    // Two concurrent claims. Without serialization both would select the same
    // lowest-id entities and double-claim them; the per-type advisory lock must
    // make their results disjoint.
    let (a, b) = tokio::join!(
        repo.claim_uncrawled(
            &db,
            EntityAutoCrawlType::Idol,
            6,
            0,
            "00000000-0000-0000-0000-000000000001",
            "https://x/"
        ),
        repo.claim_uncrawled(
            &db,
            EntityAutoCrawlType::Idol,
            6,
            0,
            "00000000-0000-0000-0000-000000000001",
            "https://x/"
        )
    );
    let a = a.unwrap();
    let b = b.unwrap();

    let set_a: HashSet<i64> = a.iter().map(|c| c.entity_id).collect();
    let set_b: HashSet<i64> = b.iter().map(|c| c.entity_id).collect();
    let overlap: Vec<i64> = set_a.intersection(&set_b).copied().collect();
    assert!(
        overlap.is_empty(),
        "concurrent claims double-claimed entities: {overlap:?}"
    );
    // Together they must not claim more than the 12 inserted candidates.
    assert!(
        set_a.len() + set_b.len() <= 12,
        "claimed more than the candidate pool: {}",
        set_a.len() + set_b.len()
    );

    // Invariant: max - min <= 1.
    let (mn, mx) = round_min_max(&db).await;
    let (mn, mx) = (mn.unwrap_or(0), mx.unwrap_or(0));
    assert!(mx - mn <= 1, "round invariant violated: min={mn} max={mx}");

    remove_test_idols(&db).await;
}

/// Model D: cancelling a still-queued task leaves the entity's round untouched
/// (cancel never changes the round) and the task becomes `cancelled`.
#[tokio::test]
async fn cancel_queued_leaves_round_unchanged() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean_entity_auto_state(&db).await;
    insert_test_idols(&db, 3).await;
    let repo = CrawlRepo;

    // Claim one entity via uncrawled scope (round stays 0 in Model D).
    let claimed = repo
        .claim_uncrawled(
            &db,
            EntityAutoCrawlType::Idol,
            1,
            0,
            "00000000-0000-0000-0000-000000000001",
            "https://x/",
        )
        .await
        .unwrap();
    let one = &claimed[0];
    assert_eq!(entity_round(&db, one.entity_id).await, Some(0));

    // Cancel the queued task -> round is unchanged (0), task is cancelled.
    repo.cancel_queued_entity_auto_task(&db, one.task_id)
        .await
        .unwrap();
    assert_eq!(
        entity_round(&db, one.entity_id).await,
        Some(0),
        "cancel must not change the round (Model D)"
    );

    // The entity is immediately re-selectable via uncrawled (round 0 = current,
    // no pending task: its last task is now cancelled).
    let reclaimed = repo
        .claim_uncrawled(
            &db,
            EntityAutoCrawlType::Idol,
            3,
            0,
            "00000000-0000-0000-0000-000000000001",
            "https://x/",
        )
        .await
        .unwrap();
    assert!(
        reclaimed.iter().any(|c| c.entity_id == one.entity_id),
        "cancelled entity must be re-selectable via uncrawled scope"
    );

    remove_test_idols(&db).await;
}

/// A failed entity stays at its round (= MIN) and is re-selectable via the
/// `failed` scope; the failed re-claim does not change the round either.
#[tokio::test]
async fn failed_entity_reclaimable_round_unchanged() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean_entity_auto_state(&db).await;
    insert_test_idols(&db, 3).await;
    let repo = CrawlRepo;
    let user = "00000000-0000-0000-0000-000000000001";

    // Uncrawled-claim (round stays 0), then mark the task failed.
    let claimed = repo
        .claim_uncrawled(&db, EntityAutoCrawlType::Idol, 1, 0, user, "https://x/")
        .await
        .unwrap();
    let first = &claimed[0];
    assert_eq!(entity_round(&db, first.entity_id).await, Some(0));
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE crawl_task SET status = 'failed', completed_at = now() WHERE id = $1",
        [first.task_id.into()],
    ))
    .await
    .unwrap();

    // Failed-scope re-claim: round stays 0 (claim is round-neutral), new task.
    let retried = repo
        .claim_failed(&db, EntityAutoCrawlType::Idol, 1, 0, user, "https://x/")
        .await
        .unwrap();
    let retry = &retried[0];
    assert_eq!(retry.entity_id, first.entity_id);
    assert_eq!(
        entity_round(&db, retry.entity_id).await,
        Some(0),
        "failed re-claim must leave the round unchanged"
    );

    remove_test_idols(&db).await;
}
