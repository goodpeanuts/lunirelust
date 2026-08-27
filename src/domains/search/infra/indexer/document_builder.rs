//! Batch construction of search documents from the primary database.

use std::collections::HashMap;

use sea_orm::{
    ColumnTrait as _, DatabaseConnection, EntityTrait as _, QueryFilter as _, QuerySelect as _,
};

use crate::domains::search::domain::model::search_document::SearchDocument;
use crate::domains::search::SearchEntityType;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Fetch all primary-database IDs for one search entity type.
pub(super) async fn fetch_entity_ids(
    db: &DatabaseConnection,
    entity_type: SearchEntityType,
) -> Result<Vec<String>, BoxError> {
    use crate::entities::{director, genre, idol, label, record, series, studio};

    let ids = match entity_type {
        SearchEntityType::Record => {
            record::Entity::find()
                .select_only()
                .column(record::Column::Id)
                .into_tuple::<String>()
                .all(db)
                .await?
        }
        SearchEntityType::Director => {
            fetch_i64_ids::<director::Entity, director::Column>(db, director::Column::Id).await?
        }
        SearchEntityType::Genre => {
            fetch_i64_ids::<genre::Entity, genre::Column>(db, genre::Column::Id).await?
        }
        SearchEntityType::Idol => {
            fetch_i64_ids::<idol::Entity, idol::Column>(db, idol::Column::Id).await?
        }
        SearchEntityType::Label => {
            fetch_i64_ids::<label::Entity, label::Column>(db, label::Column::Id).await?
        }
        SearchEntityType::Studio => {
            fetch_i64_ids::<studio::Entity, studio::Column>(db, studio::Column::Id).await?
        }
        SearchEntityType::Series => {
            fetch_i64_ids::<series::Entity, series::Column>(db, series::Column::Id).await?
        }
    };

    Ok(ids)
}

async fn fetch_i64_ids<E, C>(
    db: &DatabaseConnection,
    column: C,
) -> Result<Vec<String>, sea_orm::DbErr>
where
    E: sea_orm::EntityTrait,
    C: sea_orm::ColumnTrait,
{
    Ok(E::find()
        .select_only()
        .column(column)
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .map(|id| id.to_string())
        .collect())
}

/// Build documents for a set of IDs using one shared version.
pub(super) async fn build_documents_for_ids(
    db: &DatabaseConnection,
    entity_type: SearchEntityType,
    entity_ids: &[String],
    version: i64,
) -> Result<Vec<SearchDocument>, BoxError> {
    if entity_type == SearchEntityType::Record {
        let inputs: Vec<(String, i64)> =
            entity_ids.iter().map(|id| (id.clone(), version)).collect();
        return build_record_documents(db, &inputs).await;
    }

    build_named_documents(db, entity_type, entity_ids, version).await
}

/// Build record documents in bulk, including all searchable relation names.
pub(super) async fn build_record_documents(
    db: &DatabaseConnection,
    inputs: &[(String, i64)],
) -> Result<Vec<SearchDocument>, BoxError> {
    use crate::entities::{
        director, genre, idol, idol_participation, label, record, record_genre, series, studio,
    };

    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    let versions: HashMap<&str, i64> = inputs
        .iter()
        .map(|(id, version)| (id.as_str(), *version))
        .collect();
    let record_ids: Vec<String> = versions.keys().map(|id| (*id).to_owned()).collect();

    let records = record::Entity::find()
        .filter(record::Column::Id.is_in(record_ids.clone()))
        .all(db)
        .await?;
    if records.is_empty() {
        return Ok(Vec::new());
    }

    let director_ids: Vec<i64> = records.iter().map(|row| row.director_id).collect();
    let studio_ids: Vec<i64> = records.iter().map(|row| row.studio_id).collect();
    let label_ids: Vec<i64> = records.iter().map(|row| row.label_id).collect();
    let series_ids: Vec<i64> = records.iter().map(|row| row.series_id).collect();

    let director_map: HashMap<i64, String> = director::Entity::find()
        .filter(director::Column::Id.is_in(director_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    let studio_map: HashMap<i64, String> = studio::Entity::find()
        .filter(studio::Column::Id.is_in(studio_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    let label_map: HashMap<i64, String> = label::Entity::find()
        .filter(label::Column::Id.is_in(label_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    let series_map: HashMap<i64, String> = series::Entity::find()
        .filter(series::Column::Id.is_in(series_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();

    let genre_rows = record_genre::Entity::find()
        .filter(record_genre::Column::RecordId.is_in(record_ids.clone()))
        .find_also_related(genre::Entity)
        .all(db)
        .await?;
    let mut genres_by_record: HashMap<String, Vec<String>> = HashMap::new();
    for (relation, related) in genre_rows {
        if let Some(genre) = related {
            genres_by_record
                .entry(relation.record_id)
                .or_default()
                .push(genre.name);
        }
    }

    let idol_rows = idol_participation::Entity::find()
        .filter(idol_participation::Column::RecordId.is_in(record_ids))
        .find_also_related(idol::Entity)
        .all(db)
        .await?;
    let mut idols_by_record: HashMap<String, Vec<String>> = HashMap::new();
    for (relation, related) in idol_rows {
        if let Some(idol) = related {
            idols_by_record
                .entry(relation.record_id)
                .or_default()
                .push(idol.name);
        }
    }

    Ok(records
        .into_iter()
        .map(|row| SearchDocument {
            doc_id: format!("record__{}", row.id),
            title: row.title,
            entity_type: SearchEntityType::Record,
            entity_id: row.id.clone(),
            entity_version: versions.get(row.id.as_str()).copied().unwrap_or_default(),
            permission: row.permission,
            date: Some(row.date.to_string()),
            duration: Some(row.duration),
            director_name: director_map.get(&row.director_id).cloned(),
            studio_name: studio_map.get(&row.studio_id).cloned(),
            label_name: label_map.get(&row.label_id).cloned(),
            series_name: series_map.get(&row.series_id).cloned(),
            genre_names: Some(genres_by_record.remove(&row.id).unwrap_or_default()),
            idol_names: Some(idols_by_record.remove(&row.id).unwrap_or_default()),
            vectors: None,
        })
        .collect())
}

async fn build_named_documents(
    db: &DatabaseConnection,
    entity_type: SearchEntityType,
    entity_ids: &[String],
    version: i64,
) -> Result<Vec<SearchDocument>, BoxError> {
    use crate::entities::{director, genre, idol, label, series, studio};

    let ids = entity_ids
        .iter()
        .map(|id| id.parse::<i64>())
        .collect::<Result<Vec<_>, _>>()?;

    let values: Vec<(i64, String)> = match entity_type {
        SearchEntityType::Director => director::Entity::find()
            .filter(director::Column::Id.is_in(ids))
            .all(db)
            .await?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect(),
        SearchEntityType::Genre => genre::Entity::find()
            .filter(genre::Column::Id.is_in(ids))
            .all(db)
            .await?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect(),
        SearchEntityType::Idol => idol::Entity::find()
            .filter(idol::Column::Id.is_in(ids))
            .all(db)
            .await?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect(),
        SearchEntityType::Label => label::Entity::find()
            .filter(label::Column::Id.is_in(ids))
            .all(db)
            .await?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect(),
        SearchEntityType::Studio => studio::Entity::find()
            .filter(studio::Column::Id.is_in(ids))
            .all(db)
            .await?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect(),
        SearchEntityType::Series => series::Entity::find()
            .filter(series::Column::Id.is_in(ids))
            .all(db)
            .await?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect(),
        SearchEntityType::Record => unreachable!("record documents use build_record_documents"),
    };

    Ok(values
        .into_iter()
        .map(|(id, title)| SearchDocument {
            doc_id: format!("{}__{id}", entity_type.as_str()),
            title,
            entity_type,
            entity_id: id.to_string(),
            entity_version: version,
            permission: 0,
            date: None,
            duration: None,
            director_name: None,
            studio_name: None,
            label_name: None,
            series_name: None,
            genre_names: None,
            idol_names: None,
            vectors: None,
        })
        .collect())
}
