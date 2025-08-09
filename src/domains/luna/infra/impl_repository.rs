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
        CreateSeriesDto, CreateStudioDto, PaginatedResponse, PaginationQuery, SearchDirectorDto,
        SearchGenreDto, SearchIdolDto, SearchLabelDto, SearchRecordDto, SearchSeriesDto,
        SearchStudioDto, UpdateDirectorDto, UpdateGenreDto, UpdateIdolDto, UpdateLabelDto,
        UpdateRecordDto, UpdateSeriesDto, UpdateStudioDto,
    },
};
use crate::entities::{
    director, genre, idol, label, record, series, studio, DirectorEntity, GenreEntity, IdolEntity,
    LabelEntity, RecordEntity, SeriesEntity, StudioEntity,
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
}

// Record Repository Implementation (simplified - without complex relations for now)
pub struct RecordRepo;

#[async_trait]
impl RecordRepository for RecordRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Record>, DbErr> {
        // For now, this will return simplified records without the complex relations
        // TODO: Implement proper record loading with all relations
        let record_models = RecordEntity::find().all(db).await?;

        // Convert to domain models (this is a simplified version)
        let mut records = Vec::new();
        for record_model in record_models {
            // Load related entities
            let director = DirectorEntity::find_by_id(record_model.director_id)
                .one(db)
                .await?;
            let studio = StudioEntity::find_by_id(record_model.studio_id)
                .one(db)
                .await?;
            let label = LabelEntity::find_by_id(record_model.label_id)
                .one(db)
                .await?;
            let series = SeriesEntity::find_by_id(record_model.series_id)
                .one(db)
                .await?;

            if let (Some(director), Some(studio), Some(label), Some(series)) =
                (director, studio, label, series)
            {
                let record = Record {
                    id: record_model.id,
                    title: record_model.title,
                    date: record_model.date,
                    duration: record_model.duration,
                    director: Director::from(director),
                    studio: Studio::from(studio),
                    label: Label::from(label),
                    series: Series::from(series),
                    genres: vec![], // TODO: Load genres
                    idols: vec![],  // TODO: Load idols
                    has_links: record_model.has_links,
                    links: vec![], // TODO: Load links
                    permission: record_model.permission,
                    local_img_count: record_model.local_img_count,
                    create_time: record_model.create_time,
                    update_time: record_model.update_time,
                    creator: record_model.creator,
                    modified_by: record_model.modified_by,
                };
                records.push(record);
            }
        }

        Ok(records)
    }

    async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Option<Record>, DbErr> {
        let record_model = RecordEntity::find_by_id(id).one(db).await?;

        if let Some(record_model) = record_model {
            // Load related entities
            let director = DirectorEntity::find_by_id(record_model.director_id)
                .one(db)
                .await?;
            let studio = StudioEntity::find_by_id(record_model.studio_id)
                .one(db)
                .await?;
            let label = LabelEntity::find_by_id(record_model.label_id)
                .one(db)
                .await?;
            let series = SeriesEntity::find_by_id(record_model.series_id)
                .one(db)
                .await?;

            if let (Some(director), Some(studio), Some(label), Some(series)) =
                (director, studio, label, series)
            {
                let record = Record {
                    id: record_model.id,
                    title: record_model.title,
                    date: record_model.date,
                    duration: record_model.duration,
                    director: Director::from(director),
                    studio: Studio::from(studio),
                    label: Label::from(label),
                    series: Series::from(series),
                    genres: vec![], // TODO: Load genres
                    idols: vec![],  // TODO: Load idols
                    has_links: record_model.has_links,
                    links: vec![], // TODO: Load links
                    permission: record_model.permission,
                    local_img_count: record_model.local_img_count,
                    create_time: record_model.create_time,
                    update_time: record_model.update_time,
                    creator: record_model.creator,
                    modified_by: record_model.modified_by,
                };
                return Ok(Some(record));
            }
        }

        Ok(None)
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchRecordDto,
    ) -> Result<Vec<Record>, DbErr> {
        let mut query = RecordEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(record::Column::Id.eq(id));
        }
        if let Some(title) = search_dto.title {
            query = query.filter(record::Column::Title.contains(&title));
        }

        let record_models = query.all(db).await?;

        // Simplified version - load each record
        let mut records = Vec::new();
        for record_model in record_models {
            if let Some(record) = self.find_by_id(db, record_model.id).await? {
                records.push(record);
            }
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

        let active_record = record::ActiveModel {
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

        let result = active_record.insert(txn).await?;
        Ok(result.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: String,
        record: UpdateRecordDto,
    ) -> Result<Option<Record>, DbErr> {
        match RecordEntity::find_by_id(&id).one(txn).await? {
            Some(existing) => {
                use chrono::Utc;

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
                active_record.update_time = Set(Utc::now().date_naive());
                active_record.modified_by = Set(record.modified_by);

                let updated = active_record.update(txn).await?;

                // Return the updated record by finding it again using the connection
                // Note: We can't call find_by_id with the transaction, so we'll construct the record manually
                let director = DirectorEntity::find_by_id(updated.director_id)
                    .one(txn)
                    .await?;
                let studio = StudioEntity::find_by_id(updated.studio_id).one(txn).await?;
                let label = LabelEntity::find_by_id(updated.label_id).one(txn).await?;
                let series = SeriesEntity::find_by_id(updated.series_id).one(txn).await?;

                if let (Some(director), Some(studio), Some(label), Some(series)) =
                    (director, studio, label, series)
                {
                    let record = Record {
                        id: updated.id,
                        title: updated.title,
                        date: updated.date,
                        duration: updated.duration,
                        director: Director::from(director),
                        studio: Studio::from(studio),
                        label: Label::from(label),
                        series: Series::from(series),
                        genres: vec![], // TODO: Load genres
                        idols: vec![],  // TODO: Load idols
                        has_links: updated.has_links,
                        links: vec![], // TODO: Load links
                        permission: updated.permission,
                        local_img_count: updated.local_img_count,
                        create_time: updated.create_time,
                        update_time: updated.update_time,
                        creator: updated.creator,
                        modified_by: updated.modified_by,
                    };
                    Ok(Some(record))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: String) -> Result<bool, DbErr> {
        let result = RecordEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }
}
