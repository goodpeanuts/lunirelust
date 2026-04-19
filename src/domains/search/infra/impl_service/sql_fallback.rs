//! SQL fallback search using `PostgreSQL` LIKE queries when `MeiliSearch` is unavailable.

use sea_orm::{
    ColumnTrait as _, DatabaseConnection, EntityTrait as _, PaginatorTrait as _, QueryFilter as _,
    QuerySelect as _,
};

use crate::common::error::AppError;
use crate::domains::search::dto::{SearchResponse, SearchResultItem};
use crate::domains::search::SearchEntityType;

use super::filter_utils::parse_filters;

/// Execute SQL fallback search using `PostgreSQL` LIKE queries.
#[expect(clippy::too_many_lines)]
pub(super) async fn search_sql_fallback(
    db: &DatabaseConnection,
    query: &str,
    entity_types: &[SearchEntityType],
    filter_str: &str,
    limit: i64,
    offset: i64,
    user_permission: i32,
) -> Result<SearchResponse, AppError> {
    use crate::entities::{
        director, genre, idol, idol_participation, label, record, record_genre, series, studio,
    };
    use sea_orm::sea_query::JoinType;
    use sea_orm::RelationTrait as _;

    // Escape SQL wildcards so user input is treated literally.
    // SeaORM's contains() wraps in %...%, so we don't add our own.
    let pattern = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let wants_all = entity_types.is_empty();
    let wants = |t: &SearchEntityType| wants_all || entity_types.iter().any(|et| et == t);

    let filters = parse_filters(filter_str);
    let mut results: Vec<SearchResultItem> = Vec::new();
    let mut total: i64 = 0;
    let mut record_total: i64 = 0;

    // --- Records ---
    if wants(&SearchEntityType::Record) {
        // Match records by title, or by related entity names via subqueries.
        // This mirrors the MeiliSearch searchable attributes for SQL fallback.
        let id_match = record::Column::Id.contains(pattern.clone());
        let title_match = record::Column::Title.contains(pattern.clone());

        // Subquery: records whose director name matches
        let director_ids: Vec<i64> = director::Entity::find()
            .filter(director::Column::Name.contains(&pattern))
            .all(db)
            .await
            .map_err(AppError::DatabaseError)?
            .into_iter()
            .map(|d| d.id)
            .collect();

        let studio_ids: Vec<i64> = studio::Entity::find()
            .filter(studio::Column::Name.contains(&pattern))
            .all(db)
            .await
            .map_err(AppError::DatabaseError)?
            .into_iter()
            .map(|s| s.id)
            .collect();

        let label_ids: Vec<i64> = label::Entity::find()
            .filter(label::Column::Name.contains(&pattern))
            .all(db)
            .await
            .map_err(AppError::DatabaseError)?
            .into_iter()
            .map(|l| l.id)
            .collect();

        let series_ids: Vec<i64> = series::Entity::find()
            .filter(series::Column::Name.contains(&pattern))
            .all(db)
            .await
            .map_err(AppError::DatabaseError)?
            .into_iter()
            .map(|s| s.id)
            .collect();

        let genre_record_ids: Vec<String> =
            if !entity_types.is_empty() || wants(&SearchEntityType::Record) {
                let genre_ids: Vec<i64> = genre::Entity::find()
                    .filter(genre::Column::Name.contains(&pattern))
                    .all(db)
                    .await
                    .map_err(AppError::DatabaseError)?
                    .into_iter()
                    .map(|g| g.id)
                    .collect();
                if genre_ids.is_empty() {
                    vec![]
                } else {
                    record_genre::Entity::find()
                        .filter(record_genre::Column::GenreId.is_in(genre_ids))
                        .all(db)
                        .await
                        .map_err(AppError::DatabaseError)?
                        .into_iter()
                        .map(|rg| rg.record_id)
                        .collect()
                }
            } else {
                vec![]
            };

        let idol_record_ids: Vec<String> =
            if !entity_types.is_empty() || wants(&SearchEntityType::Record) {
                let idol_ids: Vec<i64> = idol::Entity::find()
                    .filter(idol::Column::Name.contains(&pattern))
                    .all(db)
                    .await
                    .map_err(AppError::DatabaseError)?
                    .into_iter()
                    .map(|i| i.id)
                    .collect();
                if idol_ids.is_empty() {
                    vec![]
                } else {
                    idol_participation::Entity::find()
                        .filter(idol_participation::Column::IdolId.is_in(idol_ids))
                        .all(db)
                        .await
                        .map_err(AppError::DatabaseError)?
                        .into_iter()
                        .map(|ip| ip.record_id)
                        .collect()
                }
            } else {
                vec![]
            };

        use sea_orm::Condition;
        let mut record_cond = Condition::any().add(id_match).add(title_match);
        if !director_ids.is_empty() {
            record_cond = record_cond.add(record::Column::DirectorId.is_in(director_ids));
        }
        if !studio_ids.is_empty() {
            record_cond = record_cond.add(record::Column::StudioId.is_in(studio_ids));
        }
        if !label_ids.is_empty() {
            record_cond = record_cond.add(record::Column::LabelId.is_in(label_ids));
        }
        if !series_ids.is_empty() {
            record_cond = record_cond.add(record::Column::SeriesId.is_in(series_ids));
        }
        if !genre_record_ids.is_empty() {
            record_cond = record_cond.add(record::Column::Id.is_in(genre_record_ids));
        }
        if !idol_record_ids.is_empty() {
            record_cond = record_cond.add(record::Column::Id.is_in(idol_record_ids));
        }

        let mut q = record::Entity::find()
            .filter(record_cond)
            .filter(record::Column::Permission.lte(user_permission));

        if let Some(ref director_name) = filters.director {
            let dir_id: Vec<i64> = director::Entity::find()
                .filter(director::Column::Name.eq(director_name.as_str()))
                .all(db)
                .await
                .map_err(AppError::DatabaseError)?
                .into_iter()
                .map(|d| d.id)
                .collect();
            if !dir_id.is_empty() {
                q = q.filter(record::Column::DirectorId.is_in(dir_id));
            } else {
                return Ok(SearchResponse {
                    search_mode: "sql_fallback".to_owned(),
                    total: 0,
                    limit,
                    offset,
                    results: vec![],
                });
            }
        }

        if let Some(ref studio_name) = filters.studio {
            let ids: Vec<i64> = studio::Entity::find()
                .filter(studio::Column::Name.eq(studio_name.as_str()))
                .all(db)
                .await
                .map_err(AppError::DatabaseError)?
                .into_iter()
                .map(|s| s.id)
                .collect();
            if !ids.is_empty() {
                q = q.filter(record::Column::StudioId.is_in(ids));
            } else {
                return Ok(SearchResponse {
                    search_mode: "sql_fallback".to_owned(),
                    total: 0,
                    limit,
                    offset,
                    results: vec![],
                });
            }
        }

        if let Some(ref label_name) = filters.label {
            let ids: Vec<i64> = label::Entity::find()
                .filter(label::Column::Name.eq(label_name.as_str()))
                .all(db)
                .await
                .map_err(AppError::DatabaseError)?
                .into_iter()
                .map(|l| l.id)
                .collect();
            if !ids.is_empty() {
                q = q.filter(record::Column::LabelId.is_in(ids));
            } else {
                return Ok(SearchResponse {
                    search_mode: "sql_fallback".to_owned(),
                    total: 0,
                    limit,
                    offset,
                    results: vec![],
                });
            }
        }

        if let Some(ref genre_name) = filters.genre {
            let genre_ids: Vec<i64> = genre::Entity::find()
                .filter(genre::Column::Name.eq(genre_name.as_str()))
                .all(db)
                .await
                .map_err(AppError::DatabaseError)?
                .into_iter()
                .map(|g| g.id)
                .collect();
            if !genre_ids.is_empty() {
                q = q
                    .join_rev(JoinType::InnerJoin, record_genre::Relation::Record.def())
                    .filter(record_genre::Column::GenreId.is_in(genre_ids));
            } else {
                return Ok(SearchResponse {
                    search_mode: "sql_fallback".to_owned(),
                    total: 0,
                    limit,
                    offset,
                    results: vec![],
                });
            }
        }

        if let Some(ref date_from) = filters.date_from {
            if let Ok(d) = date_from.parse::<chrono::NaiveDate>() {
                q = q.filter(record::Column::Date.gte(d));
            }
        }
        if let Some(ref date_to) = filters.date_to {
            if let Ok(d) = date_to.parse::<chrono::NaiveDate>() {
                q = q.filter(record::Column::Date.lte(d));
            }
        }

        record_total = q.clone().count(db).await.map_err(AppError::DatabaseError)? as i64;
        let found = q
            .offset(offset as u64)
            .limit(limit as u64)
            .all(db)
            .await
            .map_err(AppError::DatabaseError)?;

        for r in found {
            results.push(SearchResultItem {
                id: r.id.clone(),
                entity_type: SearchEntityType::Record,
                title: r.title,
                score: None,
                highlight: None,
                date: Some(r.date.to_string()),
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            });
        }

        total += record_total;

        // Early return only when "record" is the sole requested entity type.
        // For multi-type requests like ["record","director"], we must
        // continue to named entity searches below.
        let record_only =
            entity_types.len() == 1 && entity_types.first() == Some(&SearchEntityType::Record);
        if record_only {
            return Ok(SearchResponse {
                search_mode: "sql_fallback".to_owned(),
                total: record_total,
                limit,
                offset,
                results,
            });
        }
    }

    // --- Named entities (when wants_all or specifically requested) ---
    let remaining = (limit as usize).saturating_sub(results.len());

    // When record-specific filters are active, named-entity docs don't
    // have those fields, so skip counting/searching them.
    let has_record_filters = filters.director.is_some()
        || filters.studio.is_some()
        || filters.label.is_some()
        || filters.genre.is_some()
        || filters.date_from.is_some()
        || filters.date_to.is_some();

    // Check if there are non-record entity types that still need counting.
    let needs_entity_counts = wants(&SearchEntityType::Director)
        || wants(&SearchEntityType::Studio)
        || wants(&SearchEntityType::Label)
        || wants(&SearchEntityType::Series)
        || wants(&SearchEntityType::Genre)
        || wants(&SearchEntityType::Idol);

    if remaining == 0 && !needs_entity_counts {
        // Page is full and no other entity types are requested.
        return Ok(SearchResponse {
            search_mode: "sql_fallback".to_owned(),
            total,
            limit,
            offset,
            results,
        });
    }
    if remaining == 0 && needs_entity_counts && !has_record_filters {
        // Page is full but total must include named entity counts too.
        // Skip when record-specific filters are active (director, genre,
        // date, etc.) because named-entity docs don't have those fields.
        if wants(&SearchEntityType::Director) {
            total += director::Entity::find()
                .filter(director::Column::Name.contains(&pattern))
                .count(db)
                .await
                .map_err(AppError::DatabaseError)? as i64;
        }
        if wants(&SearchEntityType::Studio) {
            total += studio::Entity::find()
                .filter(studio::Column::Name.contains(&pattern))
                .count(db)
                .await
                .map_err(AppError::DatabaseError)? as i64;
        }
        if wants(&SearchEntityType::Label) {
            total += label::Entity::find()
                .filter(label::Column::Name.contains(&pattern))
                .count(db)
                .await
                .map_err(AppError::DatabaseError)? as i64;
        }
        if wants(&SearchEntityType::Series) {
            total += series::Entity::find()
                .filter(series::Column::Name.contains(&pattern))
                .count(db)
                .await
                .map_err(AppError::DatabaseError)? as i64;
        }
        if wants(&SearchEntityType::Genre) {
            total += genre::Entity::find()
                .filter(genre::Column::Name.contains(&pattern))
                .count(db)
                .await
                .map_err(AppError::DatabaseError)? as i64;
        }
        if wants(&SearchEntityType::Idol) {
            total += idol::Entity::find()
                .filter(idol::Column::Name.contains(&pattern))
                .count(db)
                .await
                .map_err(AppError::DatabaseError)? as i64;
        }
        return Ok(SearchResponse {
            search_mode: "sql_fallback".to_owned(),
            total,
            limit,
            offset,
            results,
        });
    }

    if has_record_filters {
        return Ok(SearchResponse {
            search_mode: "sql_fallback".to_owned(),
            total,
            limit,
            offset,
            results,
        });
    }

    // For named entity fallback, collect ALL matching entities (no per-type
    // offset), then apply the global offset+limit to the merged list.
    let mut entity_results: Vec<SearchResultItem> = Vec::new();

    if wants(&SearchEntityType::Director) {
        let q = director::Entity::find().filter(director::Column::Name.contains(&pattern));
        total += q.clone().count(db).await.map_err(AppError::DatabaseError)? as i64;
        let found = q.all(db).await.map_err(AppError::DatabaseError)?;
        for d in found {
            entity_results.push(SearchResultItem {
                id: d.id.to_string(),
                entity_type: SearchEntityType::Director,
                title: d.name,
                score: None,
                highlight: None,
                date: None,
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            });
        }
    }

    if wants(&SearchEntityType::Studio) {
        let q = studio::Entity::find().filter(studio::Column::Name.contains(&pattern));
        total += q.clone().count(db).await.map_err(AppError::DatabaseError)? as i64;
        let found = q.all(db).await.map_err(AppError::DatabaseError)?;
        for s in found {
            entity_results.push(SearchResultItem {
                id: s.id.to_string(),
                entity_type: SearchEntityType::Studio,
                title: s.name,
                score: None,
                highlight: None,
                date: None,
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            });
        }
    }

    if wants(&SearchEntityType::Label) {
        let q = label::Entity::find().filter(label::Column::Name.contains(&pattern));
        total += q.clone().count(db).await.map_err(AppError::DatabaseError)? as i64;
        let found = q.all(db).await.map_err(AppError::DatabaseError)?;
        for l in found {
            entity_results.push(SearchResultItem {
                id: l.id.to_string(),
                entity_type: SearchEntityType::Label,
                title: l.name,
                score: None,
                highlight: None,
                date: None,
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            });
        }
    }

    if wants(&SearchEntityType::Series) {
        let q = series::Entity::find().filter(series::Column::Name.contains(&pattern));
        total += q.clone().count(db).await.map_err(AppError::DatabaseError)? as i64;
        let found = q.all(db).await.map_err(AppError::DatabaseError)?;
        for s in found {
            entity_results.push(SearchResultItem {
                id: s.id.to_string(),
                entity_type: SearchEntityType::Series,
                title: s.name,
                score: None,
                highlight: None,
                date: None,
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            });
        }
    }

    if wants(&SearchEntityType::Genre) {
        let q = genre::Entity::find().filter(genre::Column::Name.contains(&pattern));
        total += q.clone().count(db).await.map_err(AppError::DatabaseError)? as i64;
        let found = q.all(db).await.map_err(AppError::DatabaseError)?;
        for g in found {
            entity_results.push(SearchResultItem {
                id: g.id.to_string(),
                entity_type: SearchEntityType::Genre,
                title: g.name,
                score: None,
                highlight: None,
                date: None,
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            });
        }
    }

    if wants(&SearchEntityType::Idol) {
        let q = idol::Entity::find().filter(idol::Column::Name.contains(&pattern));
        total += q.clone().count(db).await.map_err(AppError::DatabaseError)? as i64;
        let found = q.all(db).await.map_err(AppError::DatabaseError)?;
        for i in found {
            entity_results.push(SearchResultItem {
                id: i.id.to_string(),
                entity_type: SearchEntityType::Idol,
                title: i.name,
                score: None,
                highlight: None,
                date: None,
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            });
        }
    }

    // Apply global offset to the merged entity list, then fill remaining.
    // If offset falls within records, entities start at 0.
    // If offset extends past records, entities start at the remainder.
    let entity_offset = if results.is_empty() && record_total == 0 {
        offset as usize
    } else {
        offset.saturating_sub(record_total) as usize
    };
    if entity_offset < entity_results.len() {
        entity_results = entity_results.split_off(entity_offset);
    } else {
        entity_results.clear();
    }
    entity_results.truncate(remaining);
    results.extend(entity_results);

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
    use std::sync::Once;

    use sea_orm::{ActiveModelTrait as _, DatabaseConnection, Set};

    use crate::common::config::{setup_database, Config};
    use crate::entities::record;

    static TEST_ENV_INIT: Once = Once::new();

    fn load_test_env() {
        TEST_ENV_INIT.call_once(|| {
            dotenvy::from_filename(".env.test").expect("Failed to load .env.test");
        });
    }

    async fn setup_search_test_db() -> DatabaseConnection {
        load_test_env();
        let config = Config::from_env().expect("Failed to load config");
        setup_database(&config)
            .await
            .expect("Failed to setup test db")
    }

    #[test]
    fn test_sql_fallback_wants_all_logic() {
        // When entity_types is empty, wants_all should be true
        let entity_types: Vec<SearchEntityType> = vec![];
        let wants_all = entity_types.is_empty();
        assert!(wants_all);

        // When entity_types contains Record only, early return should happen
        let entity_types: Vec<SearchEntityType> = vec![SearchEntityType::Record];
        let wants_all = entity_types.is_empty();
        let wants_record = wants_all
            || entity_types
                .iter()
                .any(|et| et == &SearchEntityType::Record);
        assert!(!wants_all);
        assert!(wants_record);
    }

    #[test]
    fn test_sql_fallback_entity_type_filter() {
        let entity_types: Vec<SearchEntityType> = vec![];
        let wants_all = entity_types.is_empty();

        let wants = |t: &SearchEntityType| wants_all || entity_types.iter().any(|et| et == t);

        // wants_all=true should match all entity types
        assert!(wants(&SearchEntityType::Director));
        assert!(wants(&SearchEntityType::Studio));
        assert!(wants(&SearchEntityType::Label));
        assert!(wants(&SearchEntityType::Series));
        assert!(wants(&SearchEntityType::Genre));
        assert!(wants(&SearchEntityType::Idol));
        assert!(wants(&SearchEntityType::Record));

        // With explicit types, should only match those
        let entity_types: Vec<SearchEntityType> =
            vec![SearchEntityType::Director, SearchEntityType::Idol];
        let wants_all = entity_types.is_empty();
        let wants = |t: &SearchEntityType| wants_all || entity_types.iter().any(|et| et == t);
        assert!(wants(&SearchEntityType::Director));
        assert!(wants(&SearchEntityType::Idol));
        assert!(!wants(&SearchEntityType::Record));
        assert!(!wants(&SearchEntityType::Studio));
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
            "",
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
