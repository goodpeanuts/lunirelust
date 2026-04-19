use super::record_loader::{load_record_with_relations, load_records_batch, load_records_slim};
use crate::domains::luna::{
    domain::{
        CreatedNestedEntities, DirectorRepository as _, GenreRepository as _, IdolRepository as _,
        LabelRepository as _, Record, RecordRepository, SeriesRepository as _,
        StudioRepository as _,
    },
    dto::{
        CreateLinkDto, CreateRecordDto, PaginatedResponse, PaginationQuery, SearchRecordDto,
        UpdateRecordDto,
    },
    infra::{DirectorRepo, GenreRepo, IdolRepo, LabelRepo, SeriesRepo, StudioRepo},
};
use crate::entities::{idol_participation, links, record, record_genre, LinksEntity, RecordEntity};
use async_trait::async_trait;
use sea_orm::prelude::Decimal;
use sea_orm::sea_query::JoinType;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, PaginatorTrait as _, QueryFilter as _, QuerySelect as _, RelationTrait as _,
    Set,
};

// Record Repository Implementation
pub struct RecordRepo;

#[expect(clippy::too_many_lines)]
#[async_trait]
impl RecordRepository for RecordRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Record>, DbErr> {
        let record_models = RecordEntity::find().all(db).await?;
        load_records_batch(db, record_models).await
    }

    async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Option<Record>, DbErr> {
        if let Some(record_model) = RecordEntity::find_by_id(id).one(db).await? {
            let record = load_record_with_relations(db, record_model).await?;
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
        load_records_batch(db, record_models).await
    }

    async fn find_list_paginated(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchRecordDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Record>, DbErr> {
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

        let page_size = pagination
            .limit
            .filter(|&l| l > 0)
            .unwrap_or(crate::common::config::DEFAULT_PAGE_SIZE as i64)
            as u64;
        let current_offset = pagination.offset.unwrap_or(0).max(0) as u64;

        let total_items = query.clone().count(db).await?;
        let record_models = query
            .offset(current_offset)
            .limit(page_size)
            .all(db)
            .await?;
        let records = load_records_batch(db, record_models).await?;

        let next_offset = current_offset + page_size;
        let next = if next_offset < total_items {
            Some(format!("?limit={page_size}&offset={next_offset}"))
        } else {
            None
        };
        let previous = if current_offset > 0 {
            Some(format!(
                "?limit={page_size}&offset={}",
                current_offset.saturating_sub(page_size)
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_items as i64,
            next,
            previous,
            results: records,
        })
    }

    async fn create(
        &self,
        txn: &DatabaseTransaction,
        record: CreateRecordDto,
    ) -> Result<(String, CreatedNestedEntities), DbErr> {
        use crate::domains::luna::domain::CreatedNestedEntities;
        use chrono::Utc;
        let now = Utc::now().date_naive();
        let mut nested = CreatedNestedEntities::default();

        // Check if record with this ID already exists
        if let Some(_existing_record) = RecordEntity::find_by_id(&record.id).one(txn).await? {
            // Record with this ID already exists, return the existing ID
            return Ok((record.id, nested));
        }

        // Handle director creation or use default
        let director_id = if let Some(director_dto) = record.director {
            let name = director_dto.name.clone();
            let director_repo = DirectorRepo;
            let (id, _) = director_repo.create(txn, director_dto).await?;
            nested.director = Some((id, name));
            id
        } else {
            0 // Default unknown director
        };

        // Handle studio creation or use default
        let studio_id = if let Some(studio_dto) = record.studio {
            let name = studio_dto.name.clone();
            let studio_repo = StudioRepo;
            let (id, _) = studio_repo.create(txn, studio_dto).await?;
            nested.studio = Some((id, name));
            id
        } else {
            0 // Default unknown studio
        };

        // Handle label creation or use default
        let label_id = if let Some(label_dto) = record.label {
            let name = label_dto.name.clone();
            let label_repo = LabelRepo;
            let (id, _) = label_repo.create(txn, label_dto).await?;
            nested.label = Some((id, name));
            id
        } else {
            0 // Default unknown label
        };

        // Handle series creation or use default
        let series_id = if let Some(series_dto) = record.series {
            let name = series_dto.name.clone();
            let series_repo = SeriesRepo;
            let (id, _) = series_repo.create(txn, series_dto).await?;
            nested.series = Some((id, name));
            id
        } else {
            0 // Default unknown series
        };

        // Create the main record
        let record_active_model = record::ActiveModel {
            id: Set(record.id.clone()),
            title: Set(record.title),
            date: Set(record.date),
            duration: Set(record.duration),
            director_id: Set(director_id),
            studio_id: Set(studio_id),
            label_id: Set(label_id),
            series_id: Set(series_id),
            has_links: Set(record.has_links),
            permission: Set(record.permission),
            local_img_count: Set(record.local_img_count),
            create_time: Set(now),
            update_time: Set(now),
            creator: Set(record.creator),
            modified_by: Set(record.modified_by),
        };

        let inserted = record_active_model.insert(txn).await?;

        // Handle genre associations
        for genre_dto in record.genres {
            let name = genre_dto.name.clone();
            let genre_repo = GenreRepo;
            let (genre_id, _) = genre_repo.create(txn, genre_dto).await?;
            nested.genres.push((genre_id, name));

            let record_genre = record_genre::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                record_id: Set(record.id.clone()),
                genre_id: Set(genre_id),
                manual: Set(false), // Manually created association
            };
            record_genre.insert(txn).await?;
        }

        // Handle idol associations
        for idol_dto in record.idols {
            let name = idol_dto.name.clone();
            let idol_repo = IdolRepo;
            let (idol_id, _) = idol_repo.create(txn, idol_dto).await?;
            nested.idols.push((idol_id, name));

            let idol_participation = idol_participation::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                idol_id: Set(idol_id),
                record_id: Set(record.id.clone()),
                manual: Set(false), // Manually created association
            };
            idol_participation.insert(txn).await?;
        }

        // Handle links
        for link_dto in record.links {
            let link_active_model = links::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                record_id: Set(record.id.clone()),
                name: Set(link_dto.name),
                size: Set(link_dto.size.unwrap_or(Decimal::new(-1, 0))),
                date: Set(link_dto.date.unwrap_or_else(|| {
                    chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                        .expect("Failed to create default date")
                })),
                link: Set(link_dto.link.unwrap_or_default()),
                star: Set(link_dto.star.unwrap_or(false)),
            };
            link_active_model.insert(txn).await?;
        }

        Ok((inserted.id, nested))
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
            let record = load_record_with_relations(txn, updated).await?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    async fn update_record_links(
        &self,
        txn: &DatabaseTransaction,
        record_id: String,
        new_links: Vec<CreateLinkDto>,
    ) -> Result<i32, DbErr> {
        // Get existing links for the record
        let existing_links = LinksEntity::find()
            .filter(links::Column::RecordId.eq(&record_id))
            .all(txn)
            .await?;

        let mut added_count = 0;

        // Check each new link to see if it already exists
        for new_link in new_links {
            let link_exists = if let Some(ref link_url) = new_link.link {
                existing_links
                    .iter()
                    .any(|existing| existing.link == *link_url)
            } else {
                false // Skip links without URL
            };

            if !link_exists && new_link.link.is_some() {
                // Insert new link
                let link_active_model = links::ActiveModel {
                    record_id: Set(record_id.clone()),
                    name: Set(new_link.name),
                    size: Set(new_link.size.unwrap_or(Decimal::from(0))),
                    date: Set(new_link
                        .date
                        .unwrap_or_else(|| chrono::Utc::now().date_naive())),
                    link: Set(new_link.link.unwrap_or_default()),
                    star: Set(new_link.star.unwrap_or(false)),
                    ..Default::default()
                };
                link_active_model.insert(txn).await?;
                added_count += 1;
            }
        }

        // Update has_links flag if new links were added
        if added_count > 0 {
            let record = RecordEntity::find_by_id(&record_id).one(txn).await?;
            if let Some(record) = record {
                let mut active_record: record::ActiveModel = record.into();
                active_record.has_links = Set(true);
                active_record.update(txn).await?;
            }
        }

        Ok(added_count)
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: String) -> Result<bool, DbErr> {
        let result = RecordEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn find_all_slim(&self, db: &DatabaseConnection) -> Result<Vec<Record>, DbErr> {
        let record_models = RecordEntity::find().all(db).await?;
        load_records_slim(db, record_models).await
    }

    async fn find_all_ids(&self, db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
        use sea_orm::FromQueryResult;

        #[derive(FromQueryResult)]
        struct IdOnly {
            id: String,
        }

        let records: Vec<IdOnly> = RecordEntity::find()
            .select_only()
            .column(record::Column::Id)
            .into_model::<IdOnly>()
            .all(db)
            .await?;

        Ok(records.into_iter().map(|r| r.id).collect())
    }

    async fn find_by_genre_id(
        &self,
        db: &DatabaseConnection,
        genre_id: i64,
    ) -> Result<Vec<Record>, DbErr> {
        let record_models = RecordEntity::find()
            .join_rev(JoinType::InnerJoin, record_genre::Relation::Record.def())
            .filter(record_genre::Column::GenreId.eq(genre_id))
            .all(db)
            .await?;
        load_records_batch(db, record_models).await
    }

    async fn find_by_genre_id_paginated(
        &self,
        db: &DatabaseConnection,
        genre_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Record>, DbErr> {
        let query = RecordEntity::find()
            .join_rev(JoinType::InnerJoin, record_genre::Relation::Record.def())
            .filter(record_genre::Column::GenreId.eq(genre_id));

        let page_size = pagination
            .limit
            .filter(|&l| l > 0)
            .unwrap_or(crate::common::config::DEFAULT_PAGE_SIZE as i64)
            as u64;
        let current_offset = pagination.offset.unwrap_or(0).max(0) as u64;

        let total_items = query.clone().count(db).await?;
        let record_models = query
            .offset(current_offset)
            .limit(page_size)
            .all(db)
            .await?;
        let records = load_records_batch(db, record_models).await?;

        let next_offset = current_offset + page_size;
        let next = if next_offset < total_items {
            Some(format!("?limit={page_size}&offset={next_offset}"))
        } else {
            None
        };
        let previous = if current_offset > 0 {
            Some(format!(
                "?limit={page_size}&offset={}",
                current_offset.saturating_sub(page_size)
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_items as i64,
            next,
            previous,
            results: records,
        })
    }

    async fn find_by_idol_id(
        &self,
        db: &DatabaseConnection,
        idol_id: i64,
    ) -> Result<Vec<Record>, DbErr> {
        let record_models = RecordEntity::find()
            .join_rev(
                JoinType::InnerJoin,
                idol_participation::Relation::Record.def(),
            )
            .filter(idol_participation::Column::IdolId.eq(idol_id))
            .all(db)
            .await?;
        load_records_batch(db, record_models).await
    }

    async fn find_by_idol_id_paginated(
        &self,
        db: &DatabaseConnection,
        idol_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<Record>, DbErr> {
        let query = RecordEntity::find()
            .join_rev(
                JoinType::InnerJoin,
                idol_participation::Relation::Record.def(),
            )
            .filter(idol_participation::Column::IdolId.eq(idol_id));

        let page_size = pagination
            .limit
            .filter(|&l| l > 0)
            .unwrap_or(crate::common::config::DEFAULT_PAGE_SIZE as i64)
            as u64;
        let current_offset = pagination.offset.unwrap_or(0).max(0) as u64;

        let total_items = query.clone().count(db).await?;
        let record_models = query
            .offset(current_offset)
            .limit(page_size)
            .all(db)
            .await?;
        let records = load_records_batch(db, record_models).await?;

        let next_offset = current_offset + page_size;
        let next = if next_offset < total_items {
            Some(format!("?limit={page_size}&offset={next_offset}"))
        } else {
            None
        };
        let previous = if current_offset > 0 {
            Some(format!(
                "?limit={page_size}&offset={}",
                current_offset.saturating_sub(page_size)
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_items as i64,
            next,
            previous,
            results: records,
        })
    }
}
