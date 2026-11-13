//! Lightweight defaults must avoid external search work without disabling catalog writes.

use lunirelust::common::config::{setup_test_database, Config, SearchBackend};
use lunirelust::domains::luna::dto::{CreateDirectorDto, UpdateDirectorDto};
use lunirelust::domains::luna::LunaService;
use lunirelust::domains::luna::LunaServiceTrait;
use lunirelust::domains::search::{SearchService, SearchServiceTrait};
use sea_orm::{ConnectionTrait as _, DbBackend, Statement};

#[tokio::test]
async fn postgres_default_does_not_contact_enhanced_search() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let mut config = Config::from_env().expect("use .env.test");
    config.meili_url = format!("http://{}", listener.local_addr().expect("address"));
    config.vllm_embedding_url = config.meili_url.clone();
    let db = setup_test_database(&config).await.expect("test database");
    let service = <SearchService as SearchServiceTrait>::create_service(config, db);
    service
        .trigger_startup_sync()
        .await
        .expect("persist PostgreSQL startup");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(500), listener.accept())
            .await
            .is_err(),
        "PostgreSQL mode must not start a background external search connection"
    );
    assert!(!service.is_meili_ready());
}

#[tokio::test]
async fn postgres_catalog_writes_do_not_accumulate_search_events() {
    let config = Config::from_env().expect("use .env.test");
    let db = setup_test_database(&config).await.expect("test database");
    let service = <LunaService as LunaServiceTrait>::create_service(config, db.clone());
    let name = format!("outbox-off-{}", uuid::Uuid::new_v4());
    let created = service
        .director_service()
        .create_director(CreateDirectorDto {
            name: name.clone(),
            link: None,
            manual: Some(true),
        })
        .await
        .expect("create director");
    service
        .director_service()
        .update_director(
            created.id,
            UpdateDirectorDto {
                id: created.id,
                name: Some(format!("{name}-edited")),
                link: None,
                manual: None,
            },
        )
        .await
        .expect("update director");
    service
        .director_service()
        .delete_director(created.id)
        .await
        .expect("delete director");
    let id = created.id.to_string();
    let mut counts = Vec::new();
    for table in ["search_sync_events", "search_document_versions"] {
        let row = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            format!("SELECT count(*) AS n FROM {table} WHERE entity_type = 'director' AND entity_id = $1"),
            [id.clone().into()],
        )).await.expect("count search state").expect("count row");
        counts.push(row.try_get::<i64>("", "n").expect("count"));
        db.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("DELETE FROM {table} WHERE entity_type = 'director' AND entity_id = $1"),
            [id.clone().into()],
        ))
        .await
        .expect("cleanup test search state");
    }
    assert_eq!(
        counts,
        [0, 0],
        "disabled indexing must not leave events or tombstones"
    );
}

#[tokio::test]
async fn postgres_record_and_nested_entity_writes_leave_no_search_state() {
    exercise_record_and_nested_writes(SearchBackend::Postgres).await;
}

#[tokio::test]
async fn keyword_mode_preserves_atomic_outbox_and_tombstones() {
    exercise_record_and_nested_writes(SearchBackend::MeiliKeyword).await;
}

#[tokio::test]
async fn hybrid_mode_preserves_atomic_outbox_and_tombstones() {
    exercise_record_and_nested_writes(SearchBackend::MeiliHybrid).await;
}

#[expect(clippy::too_many_lines)]
async fn exercise_record_and_nested_writes(backend: SearchBackend) {
    use lunirelust::domains::luna::dto::{CreateRecordDto, UpdateRecordDto};
    use serde_json::json;

    let mut config = Config::from_env().expect("use .env.test");
    config.search_backend = backend;
    let db = setup_test_database(&config).await.expect("test database");
    let service = <LunaService as LunaServiceTrait>::create_service(config, db.clone());
    let id = format!("backend-{}", uuid::Uuid::new_v4());
    let named = json!({"name": id, "manual": true});
    let created = service
        .record_service()
        .create_record(
            serde_json::from_value::<CreateRecordDto>(json!({
                "id": id, "title": "Mode switch fixture", "date": "2026-01-01", "duration": 60,
                "director": named, "studio": named, "label": named, "series": named,
                "genres": [named], "idols": [named], "has_links": false, "links": [],
                "permission": 0, "local_img_count": 0, "creator": "test", "modified_by": "test"
            }))
            .expect("record DTO"),
        )
        .await
        .expect("create record with nested entities");
    service
        .record_service()
        .update_record(
            &id,
            serde_json::from_value::<UpdateRecordDto>(json!({
                "title": "Edited fixture", "date": "2026-01-02", "duration": 90,
                "director_id": created.director.id, "studio_id": created.studio.id,
                "label_id": created.label.id, "series_id": created.series.id,
                "genres": [{"id": created.genres[0].genre.id}],
                "idols": [{"idol_id": created.idols[0].idol.id, "manual": true}],
                "has_links": false, "links": [], "permission": 0, "local_img_count": 0,
                "modified_by": "test"
            }))
            .expect("update DTO"),
        )
        .await
        .expect("update record");
    service
        .record_service()
        .delete_record(&id)
        .await
        .expect("delete record");

    macro_rules! update_and_delete {
        ($service:ident, $update:ident, $delete:ident, $entity_id:expr) => {{
            let entity_id = $entity_id;
            service.$service().$update(entity_id,
                serde_json::from_value(json!({"id": entity_id, "name": format!("{id}-updated")}))
                    .expect("update named entity DTO")
            ).await.expect("update nested entity");
            service.$service().$delete(entity_id).await.expect("delete nested entity");
        }};
    }
    update_and_delete!(
        director_service,
        update_director,
        delete_director,
        created.director.id
    );
    update_and_delete!(
        studio_service,
        update_studio,
        delete_studio,
        created.studio.id
    );
    update_and_delete!(label_service, update_label, delete_label, created.label.id);
    update_and_delete!(
        series_service,
        update_series,
        delete_series,
        created.series.id
    );
    update_and_delete!(
        genre_service,
        update_genre,
        delete_genre,
        created.genres[0].genre.id
    );
    update_and_delete!(
        idol_service,
        update_idol,
        delete_idol,
        created.idols[0].idol.id
    );

    let mut unexpected = Vec::new();
    for (entity_type, entity_id) in [
        ("record", id),
        ("director", created.director.id.to_string()),
        ("studio", created.studio.id.to_string()),
        ("label", created.label.id.to_string()),
        ("series", created.series.id.to_string()),
        ("genre", created.genres[0].genre.id.to_string()),
        ("idol", created.idols[0].idol.id.to_string()),
    ] {
        for table in ["search_sync_events", "search_document_versions"] {
            let values = [entity_type.into(), entity_id.clone().into()];
            let row = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
                format!("SELECT count(*) AS n FROM {table} WHERE entity_type = $1 AND entity_id = $2"),
                values.clone(),
            )).await.expect("count search state").expect("count row");
            let count = row.try_get::<i64>("", "n").expect("count");
            let expected = if backend.indexing_enabled() {
                if table == "search_sync_events" {
                    3
                } else {
                    1
                }
            } else {
                0
            };
            if count != expected {
                unexpected.push(format!(
                    "{table}: {entity_type} has {count} rows, expected {expected}"
                ));
            }
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!("DELETE FROM {table} WHERE entity_type = $1 AND entity_id = $2"),
                values,
            ))
            .await
            .expect("cleanup test search state");
        }
    }
    assert!(
        unexpected.is_empty(),
        "unexpected {backend:?} search state: {unexpected:?}"
    );
}

#[tokio::test]
async fn an_empty_entity_type_filter_keeps_the_all_types_default() {
    let config = Config::from_env().expect("use .env.test");
    let db = setup_test_database(&config).await.expect("test database");
    let service = <SearchService as SearchServiceTrait>::create_service(config, db);
    let response = service
        .search(
            serde_json::from_value(serde_json::json!({
                "q": format!("no-match-{}", uuid::Uuid::new_v4()), "entity_types": "   "
            }))
            .expect("query DTO"),
            0,
        )
        .await
        .expect("empty entity types should select all types");
    assert_eq!(response.search_mode, "sql_fallback");
    assert!(response.results.is_empty());
}
