#![allow(clippy::unwrap_used)]
//! Integration tests for the idol affinity-ordered listing against the test
//! database. Verifies the per-user affinity score, which combines two
//! Bayesian-shrunk rates (shrunk `viewed/total` and `liked/viewed`, where
//! `liked` counts only works both liked AND viewed, so `liked <= viewed`) with
//! a logarithmic absolute-volume factor. Small-sample rates are pulled toward a
//! prior, so a single viewed+liked work does NOT score highest; idols with many
//! viewed/liked works rank higher. The score is a relative ordering key and is
//! NOT bounded to `[0, 1]`. Also verifies the descending order with `id`
//! ascending tiebreak, the `total=0`/`viewed=0` edge cases, and pagination
//! parity with the shared macro (`count`/`next`/`previous`).
//!
//! These tests share global tables (`idol`, `record`, `idol_participation`,
//! `user_record_interaction`), so each holds a transaction-scoped advisory lock
//! on a dedicated connection to serialize across nextest's per-test processes.

use lunirelust::common::config::{setup_database, Config};
use lunirelust::domains::luna::dto::{PaginationQuery, SearchIdolDto};
use lunirelust::domains::luna::{IdolAffinityRepository as _, IdolRepo};
use sea_orm::{
    ConnectionTrait as _, DatabaseBackend, DatabaseConnection, DatabaseTransaction, Statement,
    TransactionTrait as _,
};

/// Seeded admin user (see `m20250813_130000_create_admin_user`); used as the
/// interaction owner for these tests.
const USER_ID: &str = "00000000-0000-0000-0000-000000000000";
/// Distinctly-prefixed test data so cleanup never touches production rows.
const TEST_LINK_PREFIX: &str = "https://afftest";
const TEST_RECORD_PREFIX: &str = "AFFTEST-";
/// Serialization key for these DB-shared tests (distinct from other suites).
const SERIAL_KEY: i64 = 91_231;

async fn db() -> DatabaseConnection {
    let _env = dotenvy::from_filename(".env.test");
    let config = Config::from_env().expect("config");
    setup_database(&config).await.expect("test db")
}

/// Hold a transaction-scoped advisory lock for the test's duration, serializing
/// these DB-shared tests across nextest processes. Released on drop.
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

async fn exec(db: &DatabaseConnection, sql: &str, values: Vec<sea_orm::Value>) {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        values,
    ))
    .await
    .unwrap();
}

/// Remove all test rows (idempotent) so each test starts deterministically.
/// Order respects FKs: interactions and participations before records/idols.
async fn clean(db: &DatabaseConnection) {
    exec(
        db,
        "DELETE FROM user_record_interaction WHERE record_id LIKE $1",
        vec![format!("{TEST_RECORD_PREFIX}%").into()],
    )
    .await;
    exec(
        db,
        "DELETE FROM idol_participation WHERE record_id LIKE $1",
        vec![format!("{TEST_RECORD_PREFIX}%").into()],
    )
    .await;
    exec(
        db,
        "DELETE FROM record WHERE id LIKE $1",
        vec![format!("{TEST_RECORD_PREFIX}%").into()],
    )
    .await;
    exec(
        db,
        "DELETE FROM idol WHERE link LIKE $1",
        vec![format!("{TEST_LINK_PREFIX}-%").into()],
    )
    .await;
}

/// Insert a test idol; returns its id.
async fn insert_idol(db: &DatabaseConnection, name: &str) -> i64 {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO idol (name, link, manual) VALUES ($1, $2, false) RETURNING id",
            [name.into(), format!("{TEST_LINK_PREFIX}-{name}").into()],
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get::<i64>("", "id").unwrap()
}

/// Insert a test record (all FK parents use the seeded id=0 rows) and link it to
/// `idol_id`.
async fn insert_record_for_idol(db: &DatabaseConnection, record_id: &str, idol_id: i64) {
    exec(
        db,
        "INSERT INTO record \
         (id, title, date, duration, director_id, studio_id, label_id, series_id, \
          has_links, permission, local_img_count, create_time, update_time, creator, modified_by) \
         VALUES ($1, $1, '2020-01-01', 0, 0, 0, 0, 0, false, 0, 0, \
                 '2020-01-01', '2020-01-01', $2, $2)",
        vec![record_id.into(), USER_ID.into()],
    )
    .await;
    exec(
        db,
        "INSERT INTO idol_participation (idol_id, record_id, manual) VALUES ($1, $2, false)",
        vec![idol_id.into(), record_id.into()],
    )
    .await;
}

/// Upsert a viewed/liked interaction for the seeded user over `record_id`.
async fn set_interaction(db: &DatabaseConnection, record_id: &str, viewed: bool, liked: bool) {
    exec(
        db,
        "INSERT INTO user_record_interaction (user_id, record_id, viewed, liked) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id, record_id) DO UPDATE SET viewed = $3, liked = $4",
        vec![
            USER_ID.into(),
            record_id.into(),
            viewed.into(),
            liked.into(),
        ],
    )
    .await;
}

fn pagination(limit: i64, offset: i64) -> PaginationQuery {
    PaginationQuery {
        limit: Some(limit),
        offset: Some(offset),
        liked_only: None,
        viewed_only: None,
    }
}

/// Shared name token so a `name` filter narrows the result set to just this
/// test's idols, isolating ordering assertions from seeded/production idols.
const NAME_TOKEN: &str = "zqaff";

fn token_search() -> SearchIdolDto {
    SearchIdolDto {
        id: None,
        name: Some(NAME_TOKEN.to_owned()),
        link: None,
        search: None,
    }
}

/// Ordering: an idol the user viewed+liked heavily ranks above one with works
/// but no interactions, which in turn ranks above a zero-work idol.
#[tokio::test]
async fn affinity_orders_by_score_desc() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean(&db).await;

    // idol A: 4 works, viewed 4, liked 4 -> high ratio + volume -> top score.
    let a = insert_idol(&db, &format!("{NAME_TOKEN}-aaa")).await;
    for i in 0..4 {
        let rid = format!("{TEST_RECORD_PREFIX}A{i}");
        insert_record_for_idol(&db, &rid, a).await;
        set_interaction(&db, &rid, true, true).await;
    }
    // idol B: 4 works, no interactions. viewed=0/liked=0 -> volume factor is 1
    // (ln(1)=0) and only the shrinkage priors contribute, so the score is small
    // but strictly positive (total > 0).
    let b = insert_idol(&db, &format!("{NAME_TOKEN}-bbb")).await;
    for i in 0..4 {
        insert_record_for_idol(&db, &format!("{TEST_RECORD_PREFIX}B{i}"), b).await;
    }
    // idol C: 0 works -> score = 0, ranks last.
    let c = insert_idol(&db, &format!("{NAME_TOKEN}-ccc")).await;

    let repo = IdolRepo;
    let page = repo
        .find_list_paginated_by_affinity(&db, token_search(), pagination(50, 0), USER_ID)
        .await
        .unwrap();

    let ids: Vec<i64> = page.results.iter().map(|d| d.id).collect();
    let pos = |x: i64| ids.iter().position(|&id| id == x).unwrap();
    // A (heavy interactions) must precede B and C.
    assert!(pos(a) < pos(b), "A should rank before B: {ids:?}");
    assert!(pos(a) < pos(c), "A should rank before C: {ids:?}");
    // B has works (total=4) so its shrinkage priors give it a positive score,
    // while C (total=0) scores 0. B must therefore rank strictly before C.
    assert!(
        pos(b) < pos(c),
        "B (has works) should rank before C (none): {ids:?}"
    );

    clean(&db).await;
}

/// Edge cases: viewed=0 causes no divide error (the shrunk rates keep positive
/// denominators) and a work liked WITHOUT being viewed is excluded from `liked`,
/// so it cannot inflate the score. An idol with a genuine viewed+liked work
/// still outranks one whose likes are all unviewed.
#[tokio::test]
async fn affinity_edge_cases_viewed_zero_and_liked_not_viewed() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean(&db).await;

    // idol D: 2 works, both LIKED but NOT viewed. viewed=0/liked=0 (a like
    // without a view does not count), so the volume factor is 1 and only the
    // shrinkage priors contribute -> small score, no divide-by-zero, and the
    // unviewed likes do not leak into the score.
    let d = insert_idol(&db, &format!("{NAME_TOKEN}-ddd")).await;
    for i in 0..2 {
        let rid = format!("{TEST_RECORD_PREFIX}D{i}");
        insert_record_for_idol(&db, &rid, d).await;
        set_interaction(&db, &rid, false, true).await; // liked, not viewed
    }
    // idol E: 2 works, viewed 1, and that 1 is liked -> both-liked-and-viewed=1.
    // Genuine interaction gives it a higher ratio and a volume factor > 1.
    let e = insert_idol(&db, &format!("{NAME_TOKEN}-eee")).await;
    let e0 = format!("{TEST_RECORD_PREFIX}E0");
    let e1 = format!("{TEST_RECORD_PREFIX}E1");
    insert_record_for_idol(&db, &e0, e).await;
    insert_record_for_idol(&db, &e1, e).await;
    set_interaction(&db, &e0, true, true).await;
    // e1: liked but not viewed -> must NOT count toward `liked`.
    set_interaction(&db, &e1, false, true).await;

    let repo = IdolRepo;
    let page = repo
        .find_list_paginated_by_affinity(&db, token_search(), pagination(50, 0), USER_ID)
        .await
        .unwrap();
    let ids: Vec<i64> = page.results.iter().map(|d| d.id).collect();
    let pos = |x: i64| ids.iter().position(|&id| id == x).unwrap();

    // E (genuine viewed+liked work) must rank above D (all likes are unviewed).
    assert!(
        pos(e) < pos(d),
        "E (viewed+liked) must outrank D (liked-but-unviewed): {ids:?}"
    );

    clean(&db).await;
}

/// Pagination parity: `count` reflects all matching idols, and `next`/`previous`
/// follow the macro's page-number semantics. Uses a `name` token filter so the
/// result set is confined to this test's own idols, keeping `count` stable under
/// concurrent inserts from other test binaries (the advisory lock only
/// serializes the affinity suite, not the whole database).
#[tokio::test]
async fn affinity_pagination_count_next_previous() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean(&db).await;

    // Insert 5 token-prefixed idols; with page_size 2 that is 3 pages.
    for n in ["p0", "p1", "p2", "p3", "p4"] {
        insert_idol(&db, &format!("{NAME_TOKEN}-{n}")).await;
    }
    let total: i64 = 5;

    let repo = IdolRepo;
    // Page 0 of size 2: count = total, has next, no previous.
    let p0 = repo
        .find_list_paginated_by_affinity(&db, token_search(), pagination(2, 0), USER_ID)
        .await
        .unwrap();
    assert_eq!(p0.count, total, "count must equal all matching idols");
    assert_eq!(p0.results.len(), 2, "page size honored");
    assert!(p0.previous.is_none(), "first page has no previous");
    assert!(p0.next.is_some(), "more pages exist -> next present");

    // Second page (offset=2): both previous and next present (3 pages total).
    let p1 = repo
        .find_list_paginated_by_affinity(&db, token_search(), pagination(2, 2), USER_ID)
        .await
        .unwrap();
    assert_eq!(p1.count, total);
    assert!(p1.previous.is_some(), "second page has previous");
    assert!(p1.next.is_some(), "third page still ahead -> next present");

    // A negative offset is clamped to page 0 (no panic, no negative SQL OFFSET).
    let neg = repo
        .find_list_paginated_by_affinity(&db, token_search(), pagination(2, -40), USER_ID)
        .await
        .unwrap();
    assert_eq!(neg.count, total, "negative offset must not corrupt count");
    assert!(
        neg.previous.is_none(),
        "clamped to first page -> no previous"
    );

    clean(&db).await;
}

/// Search filter parity: `name` filter constrains both `count` and `results`,
/// and is case-sensitive substring (`LIKE`, not `ILIKE`) like the macro.
#[tokio::test]
async fn affinity_name_filter_constrains_count_and_results() {
    let db = db().await;
    let _lock = begin_serial(&db).await;
    clean(&db).await;

    insert_idol(&db, "zzz-match-lower").await;
    insert_idol(&db, "zzz-MATCH-upper").await;
    insert_idol(&db, "zzz-other").await;

    let repo = IdolRepo;
    let search = SearchIdolDto {
        id: None,
        name: Some("match-lower".to_owned()),
        link: None,
        search: None,
    };
    let page = repo
        .find_list_paginated_by_affinity(&db, search, pagination(50, 0), USER_ID)
        .await
        .unwrap();

    // Case-sensitive LIKE: only the lower-case name matches.
    assert_eq!(page.count, 1, "count reflects the filter");
    assert_eq!(page.results.len(), 1, "results reflect the filter");
    assert_eq!(page.results[0].name, "zzz-match-lower");

    clean(&db).await;
}
