use super::record_loader::{load_record_with_relations, load_records_batch, load_records_slim};
use crate::domains::luna::{
    domain::{
        CreatedNestedEntities, DirectorRepository as _, GenreRepository as _, IdolRepository as _,
        LabelRepository as _, Record, RecordRepository, SeriesRepository as _,
        StudioRepository as _,
    },
    dto::{
        CreateLinkDto, CreateRecordDto, PaginatedResponse, PaginationQuery, SearchRecordDto,
        UpdateRecordDto, UserFilter,
    },
    infra::{DirectorRepo, GenreRepo, IdolRepo, LabelRepo, SeriesRepo, StudioRepo},
};
use crate::entities::{
    idol_participation, links, record, record_genre, user_record_interaction, LinksEntity,
    RecordEntity,
};
use async_trait::async_trait;
use sea_orm::prelude::Decimal;
use sea_orm::sea_query::JoinType;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, FromQueryResult, PaginatorTrait as _, QueryFilter as _, QuerySelect as _,
    RelationTrait as _, Set,
};
use std::collections::HashSet;

/// Apply user interaction filter as INNER JOIN on `user_record_interaction`.
fn apply_user_filter(
    query: sea_orm::Select<RecordEntity>,
    user_filter: &Option<UserFilter>,
) -> sea_orm::Select<RecordEntity> {
    let Some(ref filter) = user_filter else {
        return query;
    };
    if !filter.liked_only && !filter.viewed_only {
        return query;
    }

    let mut q = query.join_rev(
        JoinType::InnerJoin,
        user_record_interaction::Relation::Record.def(),
    );
    q = q.filter(user_record_interaction::Column::UserId.eq(&filter.user_id));
    if filter.liked_only {
        q = q.filter(user_record_interaction::Column::Liked.eq(true));
    }
    if filter.viewed_only {
        q = q.filter(user_record_interaction::Column::Viewed.eq(true));
    }
    q
}

/// Build a pagination link string, appending active filter params.
fn build_page_link(
    limit: u64,
    offset: u64,
    liked_param: Option<&str>,
    viewed_param: Option<&str>,
) -> String {
    let mut parts = vec![format!("limit={limit}"), format!("offset={offset}")];
    if let Some(val) = liked_param {
        parts.push(format!("liked_only={val}"));
    }
    if let Some(val) = viewed_param {
        parts.push(format!("viewed_only={val}"));
    }
    format!("?{}", parts.join("&"))
}

/// Extract (`page_size`, `current_offset`) from pagination query.
fn resolve_pagination(pagination: &PaginationQuery) -> (u64, u64) {
    let page_size = pagination
        .limit
        .filter(|&l| l > 0)
        .unwrap_or(crate::common::config::DEFAULT_PAGE_SIZE as i64) as u64;
    let current_offset = pagination.offset.unwrap_or(0).max(0) as u64;
    (page_size, current_offset)
}

/// Build a `PaginatedResponse` from results, pagination params, and filter state.
fn build_paginated_response<T>(
    results: Vec<T>,
    total_items: u64,
    page_size: u64,
    current_offset: u64,
    liked_param: Option<&str>,
    viewed_param: Option<&str>,
) -> PaginatedResponse<T> {
    let next_offset = current_offset + page_size;
    let next = if next_offset < total_items {
        Some(build_page_link(
            page_size,
            next_offset,
            liked_param,
            viewed_param,
        ))
    } else {
        None
    };
    let previous = if current_offset > 0 {
        Some(build_page_link(
            page_size,
            current_offset.saturating_sub(page_size),
            liked_param,
            viewed_param,
        ))
    } else {
        None
    };
    PaginatedResponse {
        count: total_items as i64,
        next,
        previous,
        results,
    }
}

/// Extract liked/viewed param strings from `user_filter` for page link building.
fn filter_params(user_filter: &Option<UserFilter>) -> (Option<&str>, Option<&str>) {
    user_filter
        .as_ref()
        .map(|f| {
            (
                f.liked_only.then_some("true"),
                f.viewed_only.then_some("true"),
            )
        })
        .unwrap_or((None, None))
}

// These helpers centralize the placeholder contract shared by manual link
// writes and crawler-driven incremental backfill.
fn default_link_date() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("Failed to create default link date")
}

fn default_link_name() -> String {
    "None".to_owned()
}

fn default_link_size() -> Decimal {
    Decimal::new(-1, 0)
}

// Normalize optional link metadata before persistence so every write path uses
// the same sentinels for "unknown" values.
fn resolve_link_defaults(link: &CreateLinkDto) -> (String, Decimal, chrono::NaiveDate) {
    let name = if link.name.trim().is_empty() {
        default_link_name()
    } else {
        link.name.clone()
    };

    (
        name,
        link.size.unwrap_or_else(default_link_size),
        link.date.unwrap_or_else(default_link_date),
    )
}

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
        user_filter: Option<UserFilter>,
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

        query = apply_user_filter(query, &user_filter);

        let (page_size, current_offset) = resolve_pagination(&pagination);
        let (liked_param, viewed_param) = filter_params(&user_filter);

        let total_items = query.clone().count(db).await?;
        let record_models = query
            .offset(current_offset)
            .limit(page_size)
            .all(db)
            .await?;
        let records = load_records_batch(db, record_models).await?;

        Ok(build_paginated_response(
            records,
            total_items,
            page_size,
            current_offset,
            liked_param,
            viewed_param,
        ))
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

        // Handle links (skip empty/whitespace URLs, deduplicate by URL)
        let mut seen_links: HashSet<String> = HashSet::new();
        for link_dto in record.links {
            let trimmed = link_dto.link.trim().to_owned();
            if trimmed.is_empty() || !seen_links.insert(trimmed) {
                continue;
            }
            let (name, size, date) = resolve_link_defaults(&link_dto);
            let link_active_model = links::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                record_id: Set(record.id.clone()),
                name: Set(name),
                size: Set(size),
                date: Set(date),
                link: Set(link_dto.link),
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
            let rec = load_record_with_relations(txn, updated).await?;
            Ok(Some(rec))
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

        // Track all known URLs (DB existing + newly inserted in this batch)
        let mut known_urls: HashSet<String> =
            existing_links.iter().map(|l| l.link.clone()).collect();

        let mut changed_count = 0;

        for new_link in new_links {
            if new_link.link.trim().is_empty() {
                continue;
            }

            if let Some(existing) = existing_links.iter().find(|l| l.link == new_link.link) {
                // Backfill placeholder fields on existing link
                let default_name = default_link_name();
                let default_size = default_link_size();
                let default_date = default_link_date();

                let name_changed = existing.name == default_name
                    && !new_link.name.trim().is_empty()
                    && new_link.name != default_name;
                let size_changed = existing.size == default_size && new_link.size.is_some();
                let date_changed = existing.date == default_date && new_link.date.is_some();

                if name_changed || size_changed || date_changed {
                    let mut active: links::ActiveModel = existing.clone().into();
                    if name_changed {
                        active.name = Set(new_link.name.clone());
                    }
                    if size_changed {
                        active.size = Set(new_link
                            .size
                            .expect("size must be present when size_changed"));
                    }
                    if date_changed {
                        active.date = Set(new_link
                            .date
                            .expect("date must be present when date_changed"));
                    }
                    active.update(txn).await?;
                    changed_count += 1;
                }
            } else if known_urls.insert(new_link.link.clone()) {
                let (name, size, date) = resolve_link_defaults(&new_link);
                let link_active_model = links::ActiveModel {
                    record_id: Set(record_id.clone()),
                    name: Set(name),
                    size: Set(size),
                    date: Set(date),
                    link: Set(new_link.link),
                    star: Set(new_link.star.unwrap_or(false)),
                    ..Default::default()
                };
                link_active_model.insert(txn).await?;
                changed_count += 1;
            }
        }

        // Update has_links flag if any links were changed
        if changed_count > 0 {
            let rec = RecordEntity::find_by_id(&record_id).one(txn).await?;
            if let Some(rec) = rec {
                let mut active_record: record::ActiveModel = rec.into();
                active_record.has_links = Set(true);
                active_record.update(txn).await?;
            }
        }

        Ok(changed_count)
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: String) -> Result<bool, DbErr> {
        let result = RecordEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn find_all_slim(
        &self,
        db: &DatabaseConnection,
        user_filter: Option<UserFilter>,
    ) -> Result<Vec<Record>, DbErr> {
        let query = apply_user_filter(RecordEntity::find(), &user_filter);
        let record_models = query.all(db).await?;
        load_records_slim(db, record_models).await
    }

    async fn find_all_ids(
        &self,
        db: &DatabaseConnection,
        user_filter: Option<UserFilter>,
    ) -> Result<Vec<String>, DbErr> {
        #[derive(FromQueryResult)]
        struct IdOnly {
            id: String,
        }

        let query = apply_user_filter(RecordEntity::find(), &user_filter);
        let records: Vec<IdOnly> = query
            .select_only()
            .column(record::Column::Id)
            .into_model::<IdOnly>()
            .all(db)
            .await?;

        Ok(records.into_iter().map(|r| r.id).collect())
    }

    async fn find_ids_paginated(
        &self,
        db: &DatabaseConnection,
        pagination: PaginationQuery,
        user_filter: Option<UserFilter>,
    ) -> Result<PaginatedResponse<String>, DbErr> {
        #[derive(FromQueryResult)]
        struct IdOnly {
            id: String,
        }

        let query = apply_user_filter(RecordEntity::find(), &user_filter);
        let (page_size, current_offset) = resolve_pagination(&pagination);
        let (liked_param, viewed_param) = filter_params(&user_filter);

        let total_items = query.clone().count(db).await?;
        let records: Vec<IdOnly> = query
            .select_only()
            .column(record::Column::Id)
            .offset(current_offset)
            .limit(page_size)
            .into_model::<IdOnly>()
            .all(db)
            .await?;

        let ids: Vec<String> = records.into_iter().map(|r| r.id).collect();

        Ok(build_paginated_response(
            ids,
            total_items,
            page_size,
            current_offset,
            liked_param,
            viewed_param,
        ))
    }

    async fn find_all_slim_paginated(
        &self,
        db: &DatabaseConnection,
        pagination: PaginationQuery,
        user_filter: Option<UserFilter>,
    ) -> Result<PaginatedResponse<Record>, DbErr> {
        let query = apply_user_filter(RecordEntity::find(), &user_filter);
        let (page_size, current_offset) = resolve_pagination(&pagination);
        let (liked_param, viewed_param) = filter_params(&user_filter);

        let total_items = query.clone().count(db).await?;
        let record_models = query
            .offset(current_offset)
            .limit(page_size)
            .all(db)
            .await?;
        let records = load_records_slim(db, record_models).await?;

        Ok(build_paginated_response(
            records,
            total_items,
            page_size,
            current_offset,
            liked_param,
            viewed_param,
        ))
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
        user_filter: Option<UserFilter>,
    ) -> Result<PaginatedResponse<Record>, DbErr> {
        let query = RecordEntity::find()
            .join_rev(JoinType::InnerJoin, record_genre::Relation::Record.def())
            .filter(record_genre::Column::GenreId.eq(genre_id));
        let query = apply_user_filter(query, &user_filter);

        let (page_size, current_offset) = resolve_pagination(&pagination);
        let (liked_param, viewed_param) = filter_params(&user_filter);

        let total_items = query.clone().count(db).await?;
        let record_models = query
            .offset(current_offset)
            .limit(page_size)
            .all(db)
            .await?;
        let records = load_records_batch(db, record_models).await?;

        Ok(build_paginated_response(
            records,
            total_items,
            page_size,
            current_offset,
            liked_param,
            viewed_param,
        ))
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
        user_filter: Option<UserFilter>,
    ) -> Result<PaginatedResponse<Record>, DbErr> {
        let query = RecordEntity::find()
            .join_rev(
                JoinType::InnerJoin,
                idol_participation::Relation::Record.def(),
            )
            .filter(idol_participation::Column::IdolId.eq(idol_id));
        let query = apply_user_filter(query, &user_filter);

        let (page_size, current_offset) = resolve_pagination(&pagination);
        let (liked_param, viewed_param) = filter_params(&user_filter);

        let total_items = query.clone().count(db).await?;
        let record_models = query
            .offset(current_offset)
            .limit(page_size)
            .all(db)
            .await?;
        let records = load_records_batch(db, record_models).await?;

        Ok(build_paginated_response(
            records,
            total_items,
            page_size,
            current_offset,
            liked_param,
            viewed_param,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn resolve_link_defaults_uses_placeholder_sentinels_when_name_size_and_date_are_missing() {
        // Empty/omitted metadata should collapse to the same canonical
        // placeholders that update mode later recognizes as backfillable.
        let dto = CreateLinkDto {
            name: String::new(),
            size: None,
            date: None,
            link: "https://example.com/magnet".to_owned(),
            star: None,
        };

        let (name, size, date) = resolve_link_defaults(&dto);

        assert_eq!(name, "None");
        assert_eq!(size.to_string(), "-1");
        assert_eq!(
            date,
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("1970-01-01 is always valid")
        );
    }

    #[test]
    fn resolve_link_defaults_preserves_explicit_name_size_and_date() {
        // Real metadata must survive normalization unchanged; only placeholder
        // candidates should be rewritten.
        let dto = CreateLinkDto {
            name: "Magnet Link".to_owned(),
            size: Some(Decimal::from_str_exact("1.5").expect("valid decimal")),
            date: Some(NaiveDate::from_ymd_opt(2025, 8, 11).expect("2025-08-11 is always valid")),
            link: "https://example.com/magnet".to_owned(),
            star: Some(true),
        };

        let (name, size, date) = resolve_link_defaults(&dto);

        assert_eq!(name, "Magnet Link");
        assert_eq!(size.to_string(), "1.5");
        assert_eq!(
            date,
            NaiveDate::from_ymd_opt(2025, 8, 11).expect("2025-08-11 is always valid")
        );
    }
}
