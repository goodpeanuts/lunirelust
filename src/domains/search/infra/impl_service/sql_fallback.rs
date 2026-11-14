//! `PostgreSQL` search with bounded result loading and deterministic pagination.

use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement, Value};

use crate::common::error::AppError;
use crate::domains::search::dto::{SearchQuery, SearchResponse, SearchResultItem};
use crate::domains::search::SearchEntityType;

// Preserve the established record-first entity order across page boundaries.
const NAMED_TYPES: &[SearchEntityType] = &[
    SearchEntityType::Director,
    SearchEntityType::Studio,
    SearchEntityType::Label,
    SearchEntityType::Series,
    SearchEntityType::Genre,
    SearchEntityType::Idol,
];

/// Search catalog text and related names without loading all matching IDs into Rust.
#[expect(clippy::too_many_lines)]
pub(super) async fn search_sql_fallback(
    db: &DatabaseConnection,
    query: &str,
    entity_types: &[SearchEntityType],
    filters: &SearchQuery,
    limit: i64,
    offset: i64,
    user_permission: i32,
) -> Result<SearchResponse, AppError> {
    // User text is literal, including PostgreSQL LIKE metacharacters.
    let pattern = format!(
        "%{}%",
        query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let mut values: Vec<Value> = vec![pattern.into(), user_permission.into()];
    let wants = |kind| entity_types.is_empty() || entity_types.contains(&kind);
    let has_record_filters = filters.director.is_some()
        || filters.studio.is_some()
        || filters.label.is_some()
        || filters.genre.is_some()
        || filters.date_from.is_some()
        || filters.date_to.is_some();

    let mut branches = Vec::new();
    if wants(SearchEntityType::Record) {
        values.extend([
            filters.director.clone().into(),
            filters.studio.clone().into(),
            filters.label.clone().into(),
            filters.genre.clone().into(),
            filters.date_from.clone().into(),
            filters.date_to.clone().into(),
        ]);
        // PostgreSQL 15 cannot infer the size of a materialized candidate CTE.
        // Sparse substring matches should use primary-key lookups, not hash all
        // records. Short terms have no useful trigrams and retain planner freedom.
        let record_lookup = if query.chars().count() >= 3 {
            "JOIN LATERAL (SELECT * FROM record WHERE record.id = c.id OFFSET 0) r ON true"
        } else {
            "JOIN record r ON r.id = c.id"
        };
        branches.push(format!(
            "SELECT r.id, 'record'::text AS entity_type, r.title, r.date,
            0 AS kind_order, 0::bigint AS numeric_id
            FROM candidate_ids c {record_lookup}
            JOIN director d ON d.id = r.director_id
            JOIN studio s ON s.id = r.studio_id
            JOIN label l ON l.id = r.label_id
            WHERE r.permission <= $2
              AND ($3::text IS NULL OR d.name = $3)
              AND ($4::text IS NULL OR s.name = $4)
              AND ($5::text IS NULL OR l.name = $5)
              AND ($6::text IS NULL OR EXISTS (
                  SELECT 1 FROM record_genre rg JOIN genre g ON g.id = rg.genre_id
                  WHERE rg.record_id = r.id AND g.name = $6))
              AND ($7::text IS NULL OR r.date >= $7::text::date)
              AND ($8::text IS NULL OR r.date <= $8::text::date)"
        ));
    }
    if !has_record_filters {
        for (index, kind) in NAMED_TYPES.iter().enumerate() {
            if wants(*kind) {
                // Table identifiers come exclusively from this closed enum, never user text.
                let table = kind.as_str();
                branches.push(format!("SELECT id::text, '{table}'::text, name, NULL::date,
                    {} AS kind_order, id AS numeric_id FROM {table} WHERE name ILIKE $1 AND 0 <= $2", index + 1));
            }
        }
    }
    if branches.is_empty() {
        return Ok(SearchResponse {
            search_mode: "sql_fallback".to_owned(),
            total: 0,
            limit,
            offset,
            results: Vec::new(),
        });
    }

    // Materialize matches before ordering: otherwise a rare infix match may scan
    // the entire date index to fill a page instead of using the trigram index.
    // UNION eliminates records matching more than one name or title field.
    let candidates = if wants(SearchEntityType::Record) {
        "SELECT id FROM record WHERE id ILIKE $1 OR title ILIKE $1
         UNION SELECT r.id FROM director n JOIN record r ON r.director_id = n.id WHERE n.name ILIKE $1
         UNION SELECT r.id FROM studio n JOIN record r ON r.studio_id = n.id WHERE n.name ILIKE $1
         UNION SELECT r.id FROM label n JOIN record r ON r.label_id = n.id WHERE n.name ILIKE $1
         UNION SELECT r.id FROM series n JOIN record r ON r.series_id = n.id WHERE n.name ILIKE $1
         UNION SELECT rg.record_id FROM genre n JOIN record_genre rg ON rg.genre_id = n.id WHERE n.name ILIKE $1
         UNION SELECT ip.record_id FROM idol n JOIN idol_participation ip ON ip.idol_id = n.id WHERE n.name ILIKE $1"
    } else {
        "SELECT NULL::text AS id WHERE false"
    };
    // Keep the result schema consistent for every selected entity combination.
    let cte = format!(
        "WITH candidate_ids AS MATERIALIZED ({candidates}),
        hits(id, entity_type, title, date, kind_order, numeric_id) AS MATERIALIZED ({})",
        branches.join(" UNION ALL ")
    );
    let count = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("{cte} SELECT count(*) AS total FROM hits"),
            values.clone(),
        ))
        .await?
        .ok_or_else(|| AppError::InternalErrorWithMessage("Missing search count".to_owned()))?;
    let total: i64 = count.try_get("", "total")?;
    let limit_param = values.len() + 1;
    let offset_param = limit_param + 1;
    let mut page_values = values;
    page_values.extend([limit.into(), offset.into()]);
    let rows = db.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
        format!("{cte}, page AS (
            SELECT * FROM hits
            ORDER BY kind_order, date DESC NULLS LAST,
                CASE WHEN kind_order = 0 THEN id ELSE title END, numeric_id
            LIMIT ${limit_param} OFFSET ${offset_param})
        SELECT p.id, p.entity_type, p.title, p.date::text AS date,
            d.name AS director_name, s.name AS studio_name, l.name AS label_name, se.name AS series_name,
            CASE WHEN r.id IS NOT NULL THEN ARRAY(
                SELECT g.name FROM record_genre rg JOIN genre g ON g.id = rg.genre_id
                WHERE rg.record_id = r.id ORDER BY g.name, g.id) END AS genre_names,
            CASE WHEN r.id IS NOT NULL THEN ARRAY(
                SELECT i.name FROM idol_participation ip JOIN idol i ON i.id = ip.idol_id
                WHERE ip.record_id = r.id ORDER BY i.name, i.id) END AS idol_names
        FROM page p LEFT JOIN record r ON p.entity_type = 'record' AND r.id = p.id
        LEFT JOIN director d ON d.id = r.director_id LEFT JOIN studio s ON s.id = r.studio_id
        LEFT JOIN label l ON l.id = r.label_id LEFT JOIN series se ON se.id = r.series_id
        ORDER BY p.kind_order, p.date DESC NULLS LAST,
            CASE WHEN p.kind_order = 0 THEN p.id ELSE p.title END, p.numeric_id"), page_values)).await?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let kind: String = row.try_get("", "entity_type")?;
        let entity_type = kind.parse().map_err(|_error| {
            AppError::InternalErrorWithMessage("Invalid search entity type".to_owned())
        })?;
        results.push(SearchResultItem {
            id: row.try_get("", "id")?,
            entity_type,
            title: row.try_get("", "title")?,
            score: None,
            highlight: None,
            date: row.try_get("", "date")?,
            director_name: row.try_get("", "director_name")?,
            studio_name: row.try_get("", "studio_name")?,
            label_name: row.try_get("", "label_name")?,
            series_name: row.try_get("", "series_name")?,
            genre_names: row.try_get("", "genre_names")?,
            idol_names: row.try_get("", "idol_names")?,
        });
    }
    Ok(SearchResponse {
        search_mode: "sql_fallback".to_owned(),
        total,
        limit,
        offset,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use sea_orm::{ActiveModelTrait as _, DatabaseConnection, EntityTrait as _, Set};

    use crate::common::config::{setup_test_database, Config};
    use crate::entities::record;

    async fn setup_search_test_db() -> DatabaseConnection {
        let config = Config::from_env()
            .expect("database integration tests require the isolated environment; run `just test`");
        setup_test_database(&config)
            .await
            .expect("Failed to setup test db")
    }

    #[tokio::test]
    async fn test_sql_fallback_matches_record_id() {
        let db = setup_search_test_db().await;
        let record_id = format!("sql-fallback-record-{}", uuid::Uuid::new_v4());
        let today = chrono::Utc::now().date_naive();

        let record_model = record::ActiveModel {
            id: Set(record_id.clone()),
            title: Set("Fallback Search Regression Title".to_owned()),
            date: Set(today),
            duration: Set(7200),
            director_id: Set(0),
            studio_id: Set(0),
            label_id: Set(0),
            series_id: Set(0),
            has_links: Set(false),
            permission: Set(1),
            local_img_count: Set(0),
            create_time: Set(today),
            update_time: Set(today),
            creator: Set("sql_fallback_test".to_owned()),
            modified_by: Set("sql_fallback_test".to_owned()),
        };

        record_model
            .insert(&db)
            .await
            .expect("Failed to insert regression record");

        let response = search_sql_fallback(
            &db,
            &record_id,
            &[SearchEntityType::Record],
            &serde_json::from_value(serde_json::json!({"q": record_id})).expect("query"),
            20,
            0,
            i32::MAX,
        )
        .await;

        record::Entity::delete_by_id(&record_id)
            .exec(&db)
            .await
            .expect("Failed to clean up regression record");

        let response = response.expect("SQL fallback search should succeed");
        assert_eq!(response.search_mode, "sql_fallback");
        assert!(
            response
                .results
                .iter()
                .any(|item| item.entity_type == SearchEntityType::Record && item.id == record_id),
            "expected sql fallback search to match the record by id"
        );
    }
}
