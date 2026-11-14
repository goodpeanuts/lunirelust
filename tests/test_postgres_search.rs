//! Behavioral regressions for the `PostgreSQL` search path.

use lunirelust::common::config::{setup_test_database, Config};
use lunirelust::domains::search::dto::SearchQuery;
use lunirelust::domains::search::{SearchEntityType, SearchService, SearchServiceTrait};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement};

async fn database() -> DatabaseConnection {
    let config = Config::from_env().expect("use the isolated .env.test environment");
    setup_test_database(&config).await.expect("test database")
}

fn query(q: &str, types: &str) -> SearchQuery {
    serde_json::from_value(serde_json::json!({"q": q, "entity_types": types}))
        .expect("search query")
}

async fn insert_record(db: &DatabaseConnection, id: &str, title: &str, permission: i32) {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO record (id, title, date, duration, director_id, studio_id, label_id, series_id, has_links, permission, local_img_count, create_time, update_time, creator, modified_by) VALUES ($1, $2, '2026-01-01', 1, 0, 0, 0, 0, false, $3, 0, '2026-01-01', '2026-01-01', 'search-test', 'search-test')",
        [id.into(), title.into(), permission.into()],
    )).await.expect("insert test record");
}

async fn remove_records(db: &DatabaseConnection, prefix: &str) {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM record WHERE starts_with(id, $1)",
        [prefix.into()],
    ))
    .await
    .expect("remove test records");
}

async fn insert_named(db: &DatabaseConnection, kind: SearchEntityType, name: &str) -> String {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                "INSERT INTO {} (name, link, manual) VALUES ($1, '', false) RETURNING id",
                kind.as_str()
            ),
            [name.into()],
        ))
        .await
        .expect("insert named fixture")
        .expect("inserted ID");
    row.try_get::<i64>("", "id")
        .expect("fixture ID")
        .to_string()
}

async fn remove_named(db: &DatabaseConnection, kind: SearchEntityType, prefix: &str) {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        format!("DELETE FROM {} WHERE starts_with(name, $1)", kind.as_str()),
        [prefix.into()],
    ))
    .await
    .expect("remove named fixtures");
}

#[tokio::test]
async fn postgres_search_supports_each_named_type_without_records() {
    let db = database().await;
    let service = <SearchService as SearchServiceTrait>::create_service(
        Config::from_env().expect("test config"),
        db.clone(),
    );
    let prefix = format!("NAMED-{}", uuid::Uuid::new_v4());
    for &kind in SearchEntityType::ALL {
        if kind == SearchEntityType::Record {
            continue;
        }
        let name = format!("{prefix}-{}", kind.as_str());
        let id = insert_named(&db, kind, &name).await;
        let result = service.search(query(&prefix, kind.as_str()), 1).await;
        remove_named(&db, kind, &prefix).await;
        let page = result.expect("named-only search");
        assert_eq!(page.total, 1);
        assert_eq!(page.results.len(), 1);
        assert_eq!(page.results[0].entity_type, kind);
        assert_eq!(page.results[0].id, id);
        assert_eq!(page.results[0].title, name);
        assert!(page.results[0].date.is_none());
        let empty = service
            .search(query(&prefix, kind.as_str()), 1)
            .await
            .expect("empty named-only search");
        assert_eq!(empty.total, 0);
        assert!(empty.results.is_empty());
    }
}

#[tokio::test]
async fn postgres_search_paginates_mixed_named_types_without_records() {
    let db = database().await;
    let service = <SearchService as SearchServiceTrait>::create_service(
        Config::from_env().expect("test config"),
        db.clone(),
    );
    let prefix = format!("NAMED-PAGE-{}", uuid::Uuid::new_v4());
    let kinds = [
        SearchEntityType::Director,
        SearchEntityType::Genre,
        SearchEntityType::Idol,
    ];
    for &kind in &kinds {
        for suffix in ["Z", "A"] {
            insert_named(&db, kind, &format!("{prefix}-{suffix}")).await;
        }
    }
    let mut pages = Vec::new();
    for offset in [0, 3, 6] {
        let mut request = query(&prefix, "idol,genre,director");
        request.limit = Some(3);
        request.offset = Some(offset);
        pages.push(service.search(request, 1).await);
    }
    for &kind in &kinds {
        remove_named(&db, kind, &prefix).await;
    }
    let mut actual = Vec::new();
    for (page, offset) in pages.into_iter().zip([0, 3, 6]) {
        let page = page.expect("mixed named-only page");
        assert_eq!(page.total, 6);
        assert_eq!(page.limit, 3);
        assert_eq!(page.offset, offset);
        assert_eq!(page.results.len(), if offset < 6 { 3 } else { 0 });
        actual.extend(
            page.results
                .into_iter()
                .map(|item| (item.entity_type, item.title)),
        );
    }
    let expected: Vec<_> = kinds
        .into_iter()
        .flat_map(|kind| ["A", "Z"].map(|suffix| (kind, format!("{prefix}-{suffix}"))))
        .collect();
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn postgres_search_is_case_insensitive_literal_and_permission_filtered() {
    let db = database().await;
    let prefix = format!("CASE-{}", uuid::Uuid::new_v4());
    let visible = format!("{prefix}-VISIBLE");
    let hidden = format!("{prefix}-HIDDEN");
    let title = format!("{prefix} 中文 100%_\\ Example");
    insert_record(&db, &visible, &title, 1).await;
    insert_record(&db, &hidden, &title, 9).await;
    let config = Config::from_env().expect("test config");
    let service = <SearchService as SearchServiceTrait>::create_service(config, db.clone());
    let by_id = service
        .search(query(&visible.to_lowercase(), "record"), 1)
        .await;
    let literal = service
        .search(
            query(&format!("{prefix} 中文 100%_\\ Example"), "record"),
            1,
        )
        .await;
    let padded = service
        .search(query(&format!("  {visible}  "), "record"), 1)
        .await;
    remove_records(&db, &prefix).await;
    let by_id = by_id.expect("case-insensitive query");
    assert_eq!(by_id.total, 1, "lowercase ID must match the canonical ID");
    assert_eq!(by_id.results[0].id, visible);
    assert_eq!(literal.expect("literal wildcard query").total, 1);
    assert_eq!(padded.expect("trimmed query").total, 1);
}

#[tokio::test]
async fn postgres_search_paginates_records_then_named_entities_stably() {
    let db = database().await;
    let prefix = format!("PAGE-{}", uuid::Uuid::new_v4());
    for suffix in ["B", "A"] {
        insert_record(&db, &format!("{prefix}-{suffix}"), &prefix, 1).await;
    }
    // Insert in reverse name order so heap order cannot satisfy the expected contract.
    for suffix in ["Z", "C", "A"] {
        db.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO director (name, link, manual) VALUES ($1, '', false)",
            [format!("{prefix}-{suffix}").into()],
        ))
        .await
        .expect("insert director");
    }
    let service = <SearchService as SearchServiceTrait>::create_service(
        Config::from_env().expect("test config"),
        db.clone(),
    );
    let mut pages = Vec::new();
    for offset in [0, 2, 4, 6] {
        let mut q = query(&prefix, "record,director");
        q.limit = Some(2);
        q.offset = Some(offset);
        pages.push(service.search(q, 1).await.expect("mixed page"));
    }
    remove_records(&db, &prefix).await;
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM director WHERE starts_with(name, $1)",
        [prefix.clone().into()],
    ))
    .await
    .expect("remove directors");
    for page in &pages {
        assert_eq!(page.total, 5);
    }
    assert_eq!(pages[0].results[0].id, format!("{prefix}-A"));
    assert_eq!(pages[1].results[0].entity_type, SearchEntityType::Director);
    assert_eq!(pages[1].results[0].title, format!("{prefix}-A"));
    assert_eq!(pages[1].results[1].title, format!("{prefix}-C"));
    assert_eq!(pages[2].results[0].title, format!("{prefix}-Z"));
    assert!(pages[3].results.is_empty());
}

#[tokio::test]
async fn search_rejects_invalid_filters_instead_of_silently_broadening_results() {
    let db = database().await;
    let service = <SearchService as SearchServiceTrait>::create_service(
        Config::from_env().expect("test config"),
        db,
    );
    assert!(service
        .search(query("example", "unknown"), 1)
        .await
        .is_err());
    let mut q = query("example", "record");
    q.date_from = Some("not-a-date".to_owned());
    assert!(service.search(q, 1).await.is_err());
}

#[tokio::test]
async fn search_dates_reject_noncanonical_and_unsupported_years_as_validation_errors() {
    use lunirelust::common::error::AppError;
    let service = <SearchService as SearchServiceTrait>::create_service(
        Config::from_env().expect("test config"),
        database().await,
    );
    for date in ["2026-1-01", "0000-01-01", "-0001-01-01", "+10000-01-01"] {
        let mut q = query("example", "record");
        q.date_from = Some(date.to_owned());
        assert!(
            matches!(
                service.search(q, 1).await,
                Err(AppError::ValidationError(_))
            ),
            "{date} must fail validation before reaching a backend"
        );
    }
}
