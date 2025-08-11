use crate::entities::{
    idol_participation, links, record, record_genre, GenreEntity, IdolEntity,
    IdolParticipationEntity, LabelEntity, LinksEntity, RecordEntity, RecordGenreEntity,
    SeriesEntity, StudioEntity,
};
use crate::{
    domains::luna::{
        domain::{Director, Label, Record, RecordRepository, Series, Studio},
        dto::{CreateRecordDto, SearchRecordDto, UpdateRecordDto},
    },
    entities::DirectorEntity,
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, QueryFilter as _, Set,
};

// Record Repository Implementation
pub struct RecordRepo;

impl RecordRepo {
    /// Helper function to load a complete record with all related data
    async fn load_record_with_relations(
        &self,
        db: &DatabaseConnection,
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
                genre_opt.map(|genre| crate::domains::luna::domain::RecordGenre {
                    genre: crate::domains::luna::domain::Genre::from(genre),
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
                idol_opt.map(|idol| crate::domains::luna::domain::IdolParticipation {
                    idol: crate::domains::luna::domain::Idol::from(idol),
                    manual: ip.manual,
                })
            })
            .collect();

        // Load links
        let links_models = LinksEntity::find()
            .filter(links::Column::RecordId.eq(&record_model.id))
            .all(db)
            .await?;

        let links = links_models
            .into_iter()
            .map(crate::domains::luna::domain::Link::from)
            .collect();

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

    /// Helper function to load a complete record with all related data from a transaction
    async fn load_record_with_relations_from_txn(
        &self,
        txn: &DatabaseTransaction,
        record_model: record::Model,
    ) -> Result<Record, DbErr> {
        // Load basic relations
        let director = DirectorEntity::find_by_id(record_model.director_id)
            .one(txn)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("Director not found".to_owned()))?;

        let studio = StudioEntity::find_by_id(record_model.studio_id)
            .one(txn)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("Studio not found".to_owned()))?;

        let label = LabelEntity::find_by_id(record_model.label_id)
            .one(txn)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("Label not found".to_owned()))?;

        let series = SeriesEntity::find_by_id(record_model.series_id)
            .one(txn)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("Series not found".to_owned()))?;

        // Load genres through record_genre
        let record_genres = RecordGenreEntity::find()
            .filter(record_genre::Column::RecordId.eq(&record_model.id))
            .find_also_related(GenreEntity)
            .all(txn)
            .await?;

        let genres = record_genres
            .into_iter()
            .filter_map(|(rg, genre_opt)| {
                genre_opt.map(|genre| crate::domains::luna::domain::RecordGenre {
                    genre: crate::domains::luna::domain::Genre::from(genre),
                    manual: rg.manual,
                })
            })
            .collect();

        // Load idols through idol_participation
        let idol_participations = IdolParticipationEntity::find()
            .filter(idol_participation::Column::RecordId.eq(&record_model.id))
            .find_also_related(IdolEntity)
            .all(txn)
            .await?;

        let idols = idol_participations
            .into_iter()
            .filter_map(|(ip, idol_opt)| {
                idol_opt.map(|idol| crate::domains::luna::domain::IdolParticipation {
                    idol: crate::domains::luna::domain::Idol::from(idol),
                    manual: ip.manual,
                })
            })
            .collect();

        // Load links
        let links_models = LinksEntity::find()
            .filter(links::Column::RecordId.eq(&record_model.id))
            .all(txn)
            .await?;

        let links = links_models
            .into_iter()
            .map(crate::domains::luna::domain::Link::from)
            .collect();

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
}

#[async_trait]
impl RecordRepository for RecordRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Record>, DbErr> {
        let record_models = RecordEntity::find().all(db).await?;
        let mut records = Vec::new();

        for record_model in record_models {
            let record = self.load_record_with_relations(db, record_model).await?;
            records.push(record);
        }

        Ok(records)
    }

    async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Option<Record>, DbErr> {
        if let Some(record_model) = RecordEntity::find_by_id(id).one(db).await? {
            let record = self.load_record_with_relations(db, record_model).await?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchRecordDto,
    ) -> Result<Vec<Record>, DbErr> {
        let mut query = RecordEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(record::Column::Id.like(format!("%{id}%")));
        }
        if let Some(title) = search_dto.title {
            query = query.filter(record::Column::Title.like(format!("%{title}%")));
        }
        if let Some(director_id) = search_dto.director_id {
            query = query.filter(record::Column::DirectorId.eq(director_id));
        }
        if let Some(studio_id) = search_dto.studio_id {
            query = query.filter(record::Column::StudioId.eq(studio_id));
        }
        if let Some(label_id) = search_dto.label_id {
            query = query.filter(record::Column::LabelId.eq(label_id));
        }
        if let Some(series_id) = search_dto.series_id {
            query = query.filter(record::Column::SeriesId.eq(series_id));
        }

        let record_models = query.all(db).await?;
        let mut records = Vec::new();

        for record_model in record_models {
            let record = self.load_record_with_relations(db, record_model).await?;
            records.push(record);
        }

        Ok(records)
    }

    async fn create(
        &self,
        txn: &DatabaseTransaction,
        record: CreateRecordDto,
    ) -> Result<String, DbErr> {
        use chrono::Utc;
        let now = Utc::now().date_naive();

        let record_active_model = record::ActiveModel {
            id: Set(record.id.clone()),
            title: Set(record.title),
            date: Set(record.date),
            duration: Set(record.duration),
            director_id: Set(record.director_id),
            studio_id: Set(record.studio_id),
            label_id: Set(record.label_id),
            series_id: Set(record.series_id),
            has_links: Set(record.has_links),
            permission: Set(record.permission),
            local_img_count: Set(record.local_img_count),
            create_time: Set(now),
            update_time: Set(now),
            creator: Set(record.creator),
            modified_by: Set(record.modified_by),
        };

        let inserted = record_active_model.insert(txn).await?;
        Ok(inserted.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: String,
        record: UpdateRecordDto,
    ) -> Result<Option<Record>, DbErr> {
        if let Some(existing) = RecordEntity::find_by_id(&id).one(txn).await? {
            use chrono::Utc;
            let now = Utc::now().date_naive();

            let mut active_record: record::ActiveModel = existing.into();
            active_record.title = Set(record.title);
            active_record.date = Set(record.date);
            active_record.duration = Set(record.duration);
            active_record.director_id = Set(record.director_id);
            active_record.studio_id = Set(record.studio_id);
            active_record.label_id = Set(record.label_id);
            active_record.series_id = Set(record.series_id);
            active_record.has_links = Set(record.has_links);
            active_record.permission = Set(record.permission);
            active_record.local_img_count = Set(record.local_img_count);
            active_record.update_time = Set(now);
            active_record.modified_by = Set(record.modified_by);

            let updated = active_record.update(txn).await?;
            // Convert transaction to connection for loading relations
            let record = self
                .load_record_with_relations_from_txn(txn, updated)
                .await?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: String) -> Result<bool, DbErr> {
        let result = RecordEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }
}
