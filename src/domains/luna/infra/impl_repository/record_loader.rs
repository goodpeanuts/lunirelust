//! Record data loaders for building domain `Record` objects from database models.
//!
//! Provides single-record, full-batch, and slim-batch loading strategies to
//! avoid N+1 query patterns when assembling records with their relations.

use crate::domains::luna::domain::{
    Director, Genre, Idol, IdolParticipation, Label, Link, Record, RecordGenre, Series, Studio,
};
use crate::entities::{
    director, idol_participation, label, links, record, record_genre, series, studio,
    DirectorEntity, GenreEntity, IdolEntity, IdolParticipationEntity, LabelEntity, LinksEntity,
    RecordGenreEntity, SeriesEntity, StudioEntity,
};
use sea_orm::{ColumnTrait as _, ConnectionTrait, DbErr, EntityTrait as _, QueryFilter as _};
use std::collections::HashMap;

/// Load a single record with all related data using any connection-like type.
pub(super) async fn load_record_with_relations<C: ConnectionTrait>(
    db: &C,
    record_model: record::Model,
) -> Result<Record, DbErr> {
    // Load basic relations
    let director = DirectorEntity::find_by_id(record_model.director_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Director not found".to_owned()))?;

    let studio = StudioEntity::find_by_id(record_model.studio_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Studio not found".to_owned()))?;

    let label = LabelEntity::find_by_id(record_model.label_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Label not found".to_owned()))?;

    let series = SeriesEntity::find_by_id(record_model.series_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Series not found".to_owned()))?;

    // Load genres through record_genre
    let record_genres = RecordGenreEntity::find()
        .filter(record_genre::Column::RecordId.eq(&record_model.id))
        .find_also_related(GenreEntity)
        .all(db)
        .await?;

    let genres = record_genres
        .into_iter()
        .filter_map(|(rg, genre_opt)| {
            genre_opt.map(|genre| RecordGenre {
                genre: Genre::from(genre),
                manual: rg.manual,
            })
        })
        .collect();

    // Load idols through idol_participation
    let idol_participations = IdolParticipationEntity::find()
        .filter(idol_participation::Column::RecordId.eq(&record_model.id))
        .find_also_related(IdolEntity)
        .all(db)
        .await?;

    let idols = idol_participations
        .into_iter()
        .filter_map(|(ip, idol_opt)| {
            idol_opt.map(|idol| IdolParticipation {
                idol: Idol::from(idol),
                manual: ip.manual,
            })
        })
        .collect();

    // Load links
    let links_models = LinksEntity::find()
        .filter(links::Column::RecordId.eq(&record_model.id))
        .all(db)
        .await?;

    let links = links_models.into_iter().map(Link::from).collect();

    Ok(Record {
        id: record_model.id,
        title: record_model.title,
        date: record_model.date,
        duration: record_model.duration,
        director: Director::from(director),
        studio: Studio::from(studio),
        label: Label::from(label),
        series: Series::from(series),
        genres,
        idols,
        has_links: record_model.has_links,
        links,
        permission: record_model.permission,
        local_img_count: record_model.local_img_count,
        create_time: record_model.create_time,
        update_time: record_model.update_time,
        creator: record_model.creator,
        modified_by: record_model.modified_by,
    })
}

/// Batch-load multiple records with all related data using only ~8 queries total
/// instead of 7 queries per record (N+1 fix).
#[expect(clippy::too_many_lines)]
pub(super) async fn load_records_batch<C: ConnectionTrait>(
    db: &C,
    record_models: Vec<record::Model>,
) -> Result<Vec<Record>, DbErr> {
    if record_models.is_empty() {
        return Ok(Vec::new());
    }

    let record_ids: Vec<String> = record_models.iter().map(|m| m.id.clone()).collect();

    // Collect all foreign key IDs
    let director_ids: Vec<i64> = record_models.iter().map(|m| m.director_id).collect();
    let studio_ids: Vec<i64> = record_models.iter().map(|m| m.studio_id).collect();
    let label_ids: Vec<i64> = record_models.iter().map(|m| m.label_id).collect();
    let series_ids: Vec<i64> = record_models.iter().map(|m| m.series_id).collect();

    // Batch load directors (query 1)
    let directors: HashMap<i64, _> = DirectorEntity::find()
        .filter(director::Column::Id.is_in(director_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|d| (d.id, d))
        .collect();

    // Batch load studios (query 2)
    let studios: HashMap<i64, _> = StudioEntity::find()
        .filter(studio::Column::Id.is_in(studio_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|s| (s.id, s))
        .collect();

    // Batch load labels (query 3)
    let labels: HashMap<i64, _> = LabelEntity::find()
        .filter(label::Column::Id.is_in(label_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|l| (l.id, l))
        .collect();

    // Batch load series (query 4)
    let series_map: HashMap<i64, _> = SeriesEntity::find()
        .filter(series::Column::Id.is_in(series_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|s| (s.id, s))
        .collect();

    // Batch load genres (query 5)
    let all_record_genres = RecordGenreEntity::find()
        .filter(record_genre::Column::RecordId.is_in(record_ids.clone()))
        .find_also_related(GenreEntity)
        .all(db)
        .await?;

    let genres_by_record: HashMap<String, Vec<RecordGenre>> = {
        let mut map = HashMap::new();
        for (rg, genre_opt) in all_record_genres {
            if let Some(genre) = genre_opt {
                let entry = RecordGenre {
                    genre: Genre::from(genre),
                    manual: rg.manual,
                };
                map.entry(rg.record_id.clone())
                    .or_insert_with(Vec::new)
                    .push(entry);
            }
        }
        map
    };

    // Batch load idols (query 6)
    let all_idol_participations = IdolParticipationEntity::find()
        .filter(idol_participation::Column::RecordId.is_in(record_ids.clone()))
        .find_also_related(IdolEntity)
        .all(db)
        .await?;

    let idols_by_record: HashMap<String, Vec<IdolParticipation>> = {
        let mut map = HashMap::new();
        for (ip, idol_opt) in all_idol_participations {
            if let Some(idol) = idol_opt {
                let entry = IdolParticipation {
                    idol: Idol::from(idol),
                    manual: ip.manual,
                };
                map.entry(ip.record_id.clone())
                    .or_insert_with(Vec::new)
                    .push(entry);
            }
        }
        map
    };

    // Batch load links (query 7)
    let all_links = LinksEntity::find()
        .filter(links::Column::RecordId.is_in(record_ids))
        .all(db)
        .await?;

    let links_by_record: HashMap<String, Vec<Link>> = {
        let mut map = HashMap::new();
        for link_model in all_links {
            let link = Link::from(link_model);
            map.entry(link.record_id.clone())
                .or_insert_with(Vec::new)
                .push(link);
        }
        map
    };

    // Assemble records
    let mut records = Vec::with_capacity(record_models.len());
    for record_model in record_models {
        let director = directors
            .get(&record_model.director_id)
            .ok_or_else(|| DbErr::RecordNotFound("Director not found".to_owned()))?;
        let studio = studios
            .get(&record_model.studio_id)
            .ok_or_else(|| DbErr::RecordNotFound("Studio not found".to_owned()))?;
        let label = labels
            .get(&record_model.label_id)
            .ok_or_else(|| DbErr::RecordNotFound("Label not found".to_owned()))?;
        let series = series_map
            .get(&record_model.series_id)
            .ok_or_else(|| DbErr::RecordNotFound("Series not found".to_owned()))?;

        let genres = genres_by_record
            .get(&record_model.id)
            .cloned()
            .unwrap_or_default();
        let idols = idols_by_record
            .get(&record_model.id)
            .cloned()
            .unwrap_or_default();
        let links = links_by_record
            .get(&record_model.id)
            .cloned()
            .unwrap_or_default();

        records.push(Record {
            id: record_model.id,
            title: record_model.title,
            date: record_model.date,
            duration: record_model.duration,
            director: Director::from(director.clone()),
            studio: Studio::from(studio.clone()),
            label: Label::from(label.clone()),
            series: Series::from(series.clone()),
            genres,
            idols,
            has_links: record_model.has_links,
            links,
            permission: record_model.permission,
            local_img_count: record_model.local_img_count,
            create_time: record_model.create_time,
            update_time: record_model.update_time,
            creator: record_model.creator,
            modified_by: record_model.modified_by,
        });
    }

    Ok(records)
}

/// Batch-load records with only basic fields + direct FK relations
/// (director/studio/label/series), skipping genres/idols/links.
pub(super) async fn load_records_slim<C: ConnectionTrait>(
    db: &C,
    record_models: Vec<record::Model>,
) -> Result<Vec<Record>, DbErr> {
    if record_models.is_empty() {
        return Ok(Vec::new());
    }

    // Collect all foreign key IDs
    let director_ids: Vec<i64> = record_models.iter().map(|m| m.director_id).collect();
    let studio_ids: Vec<i64> = record_models.iter().map(|m| m.studio_id).collect();
    let label_ids: Vec<i64> = record_models.iter().map(|m| m.label_id).collect();
    let series_ids: Vec<i64> = record_models.iter().map(|m| m.series_id).collect();

    // Batch load directors (query 1)
    let directors: HashMap<i64, _> = DirectorEntity::find()
        .filter(director::Column::Id.is_in(director_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|d| (d.id, d))
        .collect();

    // Batch load studios (query 2)
    let studios: HashMap<i64, _> = StudioEntity::find()
        .filter(studio::Column::Id.is_in(studio_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|s| (s.id, s))
        .collect();

    // Batch load labels (query 3)
    let labels: HashMap<i64, _> = LabelEntity::find()
        .filter(label::Column::Id.is_in(label_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|l| (l.id, l))
        .collect();

    // Batch load series (query 4)
    let series_map: HashMap<i64, _> = SeriesEntity::find()
        .filter(series::Column::Id.is_in(series_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|s| (s.id, s))
        .collect();

    // Assemble records — genres/idols/links left empty for slim mode
    let mut records = Vec::with_capacity(record_models.len());
    for record_model in record_models {
        let director = directors
            .get(&record_model.director_id)
            .ok_or_else(|| DbErr::RecordNotFound("Director not found".to_owned()))?;
        let studio = studios
            .get(&record_model.studio_id)
            .ok_or_else(|| DbErr::RecordNotFound("Studio not found".to_owned()))?;
        let label = labels
            .get(&record_model.label_id)
            .ok_or_else(|| DbErr::RecordNotFound("Label not found".to_owned()))?;
        let series = series_map
            .get(&record_model.series_id)
            .ok_or_else(|| DbErr::RecordNotFound("Series not found".to_owned()))?;

        records.push(Record {
            id: record_model.id,
            title: record_model.title,
            date: record_model.date,
            duration: record_model.duration,
            director: Director::from(director.clone()),
            studio: Studio::from(studio.clone()),
            label: Label::from(label.clone()),
            series: Series::from(series.clone()),
            genres: Vec::new(),
            idols: Vec::new(),
            has_links: record_model.has_links,
            links: Vec::new(),
            permission: record_model.permission,
            local_img_count: record_model.local_img_count,
            create_time: record_model.create_time,
            update_time: record_model.update_time,
            creator: record_model.creator,
            modified_by: record_model.modified_by,
        });
    }

    Ok(records)
}
