use crate::domains::luna::{
    domain::{
        model::{Director, Genre, Idol, Label, Record, Series, Studio},
        repository::{
            DirectorRepository, GenreRepository, IdolRepository, LabelRepository, RecordRepository,
            SeriesRepository, StudioRepository,
        },
    },
    dto::luna_dto::{
        CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateRecordDto,
        CreateSeriesDto, CreateStudioDto, EntityCountDto, PaginatedResponse, PaginationQuery,
        SearchDirectorDto, SearchGenreDto, SearchIdolDto, SearchLabelDto, SearchRecordDto,
        SearchSeriesDto, SearchStudioDto, UpdateDirectorDto, UpdateGenreDto, UpdateIdolDto,
        UpdateLabelDto, UpdateRecordDto, UpdateSeriesDto, UpdateStudioDto,
    },
};
use crate::entities::{
    director, genre, idol, idol_participation, label, links, record, record_genre, series, studio,
    DirectorEntity, GenreEntity, IdolEntity, IdolParticipationEntity, LabelEntity, LinksEntity,
    RecordEntity, RecordGenreEntity, SeriesEntity, StudioEntity,
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, PaginatorTrait as _, QueryFilter as _, Set,
};

pub struct DirectorRepo;

#[async_trait]
impl DirectorRepository for DirectorRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Director>, DbErr> {
        let directors = DirectorEntity::find().all(db).await?;
        Ok(directors.into_iter().map(Director::from).collect())
    }

    async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Option<Director>, DbErr> {
        let director = DirectorEntity::find_by_id(id).one(db).await?;
        Ok(director.map(Director::from))
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchDirectorDto,
    ) -> Result<Vec<Director>, DbErr> {
        let mut query = DirectorEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(director::Column::Id.eq(id));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(director::Column::Name.like(format!("%{name}%")));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(director::Column::Link.like(format!("%{link}%")));
        }

        let directors = query.all(db).await?;
        Ok(directors.into_iter().map(Director::from).collect())
    }

    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchDirectorDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Director>, DbErr> {
        let mut query = DirectorEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(director::Column::Id.eq(id));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(director::Column::Name.like(format!("%{name}%")));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(director::Column::Link.like(format!("%{link}%")));
        }

        let page_size = pagination.limit.unwrap_or(20) as u64;
        let page_num = (pagination.offset.unwrap_or(0) / page_size as i64) as u64;

        let paginator = query.paginate(db, page_size);
        let total_items = paginator.num_items().await?;
        let total_pages = paginator.num_pages().await?;

        let directors = paginator.fetch_page(page_num).await?;
        let director_models: Vec<Director> = directors.into_iter().map(Director::from).collect();

        let next = if page_num + 1 < total_pages {
            Some(format!(
                "?limit={}&offset={}",
                page_size,
                (page_num + 1) * page_size
            ))
        } else {
            None
        };

        let previous = if page_num > 0 {
            Some(format!(
                "?limit={}&offset={}",
                page_size,
                (page_num - 1) * page_size
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_items as i64,
            next,
            previous,
            results: director_models,
        })
    }

    async fn create(
        &self,
        txn: &DatabaseTransaction,
        director: CreateDirectorDto,
    ) -> Result<i64, DbErr> {
        let director_active_model = director::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            name: Set(director.name),
            link: Set(director.link),
        };

        let inserted = director_active_model.insert(txn).await?;
        Ok(inserted.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        director: UpdateDirectorDto,
    ) -> Result<Option<Director>, DbErr> {
        let existing_director = DirectorEntity::find_by_id(id).one(txn).await?;

        if let Some(existing) = existing_director {
            let mut director_active_model: director::ActiveModel = existing.into();
            director_active_model.name = Set(director.name);
            director_active_model.link = Set(director.link);

            let updated_director = director_active_model.update(txn).await?;
            return Ok(Some(Director::from(updated_director)));
        }

        Ok(None)
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
        let result = DirectorEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn get_director_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr> {
        let directors = DirectorEntity::find().all(db).await?;
        let mut result = Vec::new();

        for director in directors {
            let count = RecordEntity::find()
                .filter(record::Column::DirectorId.eq(director.id))
                .count(db)
                .await? as i64;

            result.push(EntityCountDto {
                id: director.id,
                name: director.name,
                count,
            });
        }

        // Sort by count descending
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}

pub struct GenreRepo;

#[async_trait]
impl GenreRepository for GenreRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Genre>, DbErr> {
        let genres = GenreEntity::find().all(db).await?;
        Ok(genres.into_iter().map(Genre::from).collect())
    }

    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Genre>, DbErr> {
        let genre = GenreEntity::find_by_id(id).one(db).await?;
        Ok(genre.map(Genre::from))
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchGenreDto,
    ) -> Result<Vec<Genre>, DbErr> {
        let mut query = GenreEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(genre::Column::Id.eq(id));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(genre::Column::Name.like(format!("%{name}%")));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(genre::Column::Link.like(format!("%{link}%")));
        }

        let results = query.all(db).await?;
        Ok(results.into_iter().map(Genre::from).collect())
    }

    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchGenreDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Genre>, DbErr> {
        let mut query = GenreEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(genre::Column::Id.eq(id));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(genre::Column::Name.like(format!("%{name}%")));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(genre::Column::Link.like(format!("%{link}%")));
        }

        let page_size = pagination.limit.unwrap_or(20) as u64;
        let page_num = (pagination.offset.unwrap_or(0) / page_size as i64) as u64;

        let paginator = query.paginate(db, page_size);
        let total_items = paginator.num_items().await?;
        let total_pages = paginator.num_pages().await?;

        let genres = paginator.fetch_page(page_num).await?;
        let genre_models: Vec<Genre> = genres.into_iter().map(Genre::from).collect();

        let next = if page_num + 1 < total_pages {
            Some(format!(
                "?limit={}&offset={}",
                page_size,
                (page_num + 1) * page_size
            ))
        } else {
            None
        };

        let previous = if page_num > 0 {
            Some(format!(
                "?limit={}&offset={}",
                page_size,
                (page_num - 1) * page_size
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_items as i64,
            next,
            previous,
            results: genre_models,
        })
    }

    async fn create(&self, txn: &DatabaseTransaction, genre: CreateGenreDto) -> Result<i64, DbErr> {
        let genre_active_model = genre::ActiveModel {
            name: Set(genre.name),
            link: Set(genre.link),
            ..Default::default()
        };

        let result = genre_active_model.insert(txn).await?;
        Ok(result.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        genre: UpdateGenreDto,
    ) -> Result<Option<Genre>, DbErr> {
        let existing_genre = GenreEntity::find_by_id(id).one(txn).await?;

        if let Some(existing) = existing_genre {
            let mut genre_active_model: genre::ActiveModel = existing.into();
            genre_active_model.name = Set(genre.name);
            genre_active_model.link = Set(genre.link);

            let updated_genre = genre_active_model.update(txn).await?;
            return Ok(Some(Genre::from(updated_genre)));
        }

        Ok(None)
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
        let result = GenreEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn get_genre_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr> {
        let genres = GenreEntity::find().all(db).await?;
        let mut result = Vec::new();

        for genre in genres {
            let count = RecordGenreEntity::find()
                .filter(record_genre::Column::GenreId.eq(genre.id))
                .count(db)
                .await? as i64;

            result.push(EntityCountDto {
                id: genre.id,
                name: genre.name,
                count,
            });
        }

        // Sort by count descending
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}

pub struct LabelRepo;

#[async_trait]
impl LabelRepository for LabelRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Label>, DbErr> {
        let labels = LabelEntity::find().all(db).await?;
        Ok(labels.into_iter().map(Label::from).collect())
    }

    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Label>, DbErr> {
        let label = LabelEntity::find_by_id(id).one(db).await?;
        Ok(label.map(Label::from))
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchLabelDto,
    ) -> Result<Vec<Label>, DbErr> {
        let mut query = LabelEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(label::Column::Id.eq(id));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(label::Column::Name.like(format!("%{name}%")));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(label::Column::Link.like(format!("%{link}%")));
        }

        let results = query.all(db).await?;
        Ok(results.into_iter().map(Label::from).collect())
    }

    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchLabelDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Label>, DbErr> {
        let mut query = LabelEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(label::Column::Id.eq(id));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(label::Column::Name.like(format!("%{name}%")));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(label::Column::Link.like(format!("%{link}%")));
        }

        let page_size = pagination.limit.unwrap_or(20) as u64;
        let page_num = (pagination.offset.unwrap_or(0) / page_size as i64) as u64;

        let paginator = query.paginate(db, page_size);
        let total_items = paginator.num_items().await?;
        let total_pages = paginator.num_pages().await?;

        let labels = paginator.fetch_page(page_num).await?;
        let label_models: Vec<Label> = labels.into_iter().map(Label::from).collect();

        let next = if page_num + 1 < total_pages {
            Some(format!(
                "?limit={}&offset={}",
                page_size,
                (page_num + 1) * page_size
            ))
        } else {
            None
        };

        let previous = if page_num > 0 {
            Some(format!(
                "?limit={}&offset={}",
                page_size,
                (page_num - 1) * page_size
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_items as i64,
            next,
            previous,
            results: label_models,
        })
    }

    async fn create(&self, txn: &DatabaseTransaction, label: CreateLabelDto) -> Result<i64, DbErr> {
        let label_active_model = label::ActiveModel {
            name: Set(label.name),
            link: Set(label.link),
            ..Default::default()
        };

        let result = label_active_model.insert(txn).await?;
        Ok(result.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        label: UpdateLabelDto,
    ) -> Result<Option<Label>, DbErr> {
        let existing_label = LabelEntity::find_by_id(id).one(txn).await?;

        if let Some(existing) = existing_label {
            let mut label_active_model: label::ActiveModel = existing.into();
            label_active_model.name = Set(label.name);
            label_active_model.link = Set(label.link);

            let updated_label = label_active_model.update(txn).await?;
            return Ok(Some(Label::from(updated_label)));
        }

        Ok(None)
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
        let result = LabelEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn get_label_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr> {
        let labels = LabelEntity::find().all(db).await?;
        let mut result = Vec::new();

        for label in labels {
            let count = RecordEntity::find()
                .filter(record::Column::LabelId.eq(label.id))
                .count(db)
                .await? as i64;

            result.push(EntityCountDto {
                id: label.id,
                name: label.name,
                count,
            });
        }

        // Sort by count descending
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}

// Studio Repository Implementation
pub struct StudioRepo;

#[async_trait]
impl StudioRepository for StudioRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Studio>, DbErr> {
        let studio_models = StudioEntity::find().all(db).await?;
        Ok(studio_models.into_iter().map(Studio::from).collect())
    }

    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Studio>, DbErr> {
        let studio_model = StudioEntity::find_by_id(id).one(db).await?;
        Ok(studio_model.map(Studio::from))
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchStudioDto,
    ) -> Result<Vec<Studio>, DbErr> {
        let mut query = StudioEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(studio::Column::Id.eq(id));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(studio::Column::Name.like(format!("%{name}%")));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query = query.filter(studio::Column::Link.like(format!("%{link}%")));
        }

        let studio_models = query.all(db).await?;
        Ok(studio_models.into_iter().map(Studio::from).collect())
    }

    async fn create(
        &self,
        txn: &DatabaseTransaction,
        create_dto: CreateStudioDto,
    ) -> Result<i64, DbErr> {
        let studio = studio::ActiveModel {
            name: Set(create_dto.name),
            link: Set(create_dto.link),
            ..Default::default()
        };

        let studio_model = studio.insert(txn).await?;
        Ok(studio_model.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        update_dto: UpdateStudioDto,
    ) -> Result<Option<Studio>, DbErr> {
        let studio = studio::ActiveModel {
            id: Set(id),
            name: Set(update_dto.name),
            link: Set(update_dto.link),
        };

        match studio.update(txn).await {
            Ok(studio_model) => Ok(Some(Studio::from(studio_model))),
            Err(_) => Ok(None),
        }
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
        let result = StudioEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn get_studio_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr> {
        let studios = StudioEntity::find().all(db).await?;
        let mut result = Vec::new();

        for studio in studios {
            let count = RecordEntity::find()
                .filter(record::Column::StudioId.eq(studio.id))
                .count(db)
                .await? as i64;

            result.push(EntityCountDto {
                id: studio.id,
                name: studio.name,
                count,
            });
        }

        // Sort by count descending
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}

// Series Repository Implementation
pub struct SeriesRepo;

#[async_trait]
impl SeriesRepository for SeriesRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Series>, DbErr> {
        let series_models = SeriesEntity::find().all(db).await?;
        Ok(series_models.into_iter().map(Series::from).collect())
    }

    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Series>, DbErr> {
        let series = SeriesEntity::find_by_id(id).one(db).await?;
        Ok(series.map(Series::from))
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchSeriesDto,
    ) -> Result<Vec<Series>, DbErr> {
        let mut query = SeriesEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(series::Column::Id.eq(id));
        }
        if let Some(name) = search_dto.name {
            query = query.filter(series::Column::Name.contains(&name));
        }
        if let Some(link) = search_dto.link {
            query = query.filter(series::Column::Link.contains(&link));
        }

        let series_models = query.all(db).await?;
        Ok(series_models.into_iter().map(Series::from).collect())
    }

    async fn create(
        &self,
        txn: &DatabaseTransaction,
        series: CreateSeriesDto,
    ) -> Result<i64, DbErr> {
        let active_series = series::ActiveModel {
            name: Set(series.name),
            link: Set(series.link),
            ..Default::default()
        };

        let result = active_series.insert(txn).await?;
        Ok(result.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        series: UpdateSeriesDto,
    ) -> Result<Option<Series>, DbErr> {
        match SeriesEntity::find_by_id(id).one(txn).await? {
            Some(existing) => {
                let mut active_series: series::ActiveModel = existing.into();
                active_series.name = Set(series.name);
                active_series.link = Set(series.link);

                let updated = active_series.update(txn).await?;
                Ok(Some(Series::from(updated)))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
        let result = SeriesEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn get_series_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr> {
        let series_list = SeriesEntity::find().all(db).await?;
        let mut result = Vec::new();

        for series in series_list {
            let count = RecordEntity::find()
                .filter(record::Column::SeriesId.eq(series.id))
                .count(db)
                .await? as i64;

            result.push(EntityCountDto {
                id: series.id,
                name: series.name,
                count,
            });
        }

        // Sort by count descending
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}

// Idol Repository Implementation
pub struct IdolRepo;

#[async_trait]
impl IdolRepository for IdolRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Idol>, DbErr> {
        let idol_models = IdolEntity::find().all(db).await?;
        Ok(idol_models.into_iter().map(Idol::from).collect())
    }

    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Idol>, DbErr> {
        let idol = IdolEntity::find_by_id(id).one(db).await?;
        Ok(idol.map(Idol::from))
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchIdolDto,
    ) -> Result<Vec<Idol>, DbErr> {
        let mut query = IdolEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(idol::Column::Id.eq(id));
        }
        if let Some(name) = search_dto.name {
            query = query.filter(idol::Column::Name.contains(&name));
        }
        if let Some(link) = search_dto.link {
            query = query.filter(idol::Column::Link.contains(&link));
        }

        let idol_models = query.all(db).await?;
        Ok(idol_models.into_iter().map(Idol::from).collect())
    }

    async fn create(&self, txn: &DatabaseTransaction, idol: CreateIdolDto) -> Result<i64, DbErr> {
        let active_idol = idol::ActiveModel {
            name: Set(idol.name),
            link: Set(idol.link),
            ..Default::default()
        };

        let result = active_idol.insert(txn).await?;
        Ok(result.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        idol: UpdateIdolDto,
    ) -> Result<Option<Idol>, DbErr> {
        match IdolEntity::find_by_id(id).one(txn).await? {
            Some(existing) => {
                let mut active_idol: idol::ActiveModel = existing.into();
                active_idol.name = Set(idol.name);
                active_idol.link = Set(idol.link);

                let updated = active_idol.update(txn).await?;
                Ok(Some(Idol::from(updated)))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
        let result = IdolEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn get_idol_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr> {
        let idols = IdolEntity::find().all(db).await?;
        let mut result = Vec::new();

        for idol in idols {
            let count = IdolParticipationEntity::find()
                .filter(idol_participation::Column::IdolId.eq(idol.id))
                .count(db)
                .await? as i64;

            result.push(EntityCountDto {
                id: idol.id,
                name: idol.name,
                count,
            });
        }

        // Sort by count descending
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}

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
                genre_opt.map(|genre| crate::domains::luna::domain::model::RecordGenre {
                    genre: crate::domains::luna::domain::model::Genre::from(genre),
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
                idol_opt.map(
                    |idol| crate::domains::luna::domain::model::IdolParticipation {
                        idol: crate::domains::luna::domain::model::Idol::from(idol),
                        manual: ip.manual,
                    },
                )
            })
            .collect();

        // Load links
        let links_models = LinksEntity::find()
            .filter(links::Column::RecordId.eq(&record_model.id))
            .all(db)
            .await?;

        let links = links_models
            .into_iter()
            .map(crate::domains::luna::domain::model::Link::from)
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
                genre_opt.map(|genre| crate::domains::luna::domain::model::RecordGenre {
                    genre: crate::domains::luna::domain::model::Genre::from(genre),
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
                idol_opt.map(
                    |idol| crate::domains::luna::domain::model::IdolParticipation {
                        idol: crate::domains::luna::domain::model::Idol::from(idol),
                        manual: ip.manual,
                    },
                )
            })
            .collect();

        // Load links
        let links_models = LinksEntity::find()
            .filter(links::Column::RecordId.eq(&record_model.id))
            .all(txn)
            .await?;

        let links = links_models
            .into_iter()
            .map(crate::domains::luna::domain::model::Link::from)
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
