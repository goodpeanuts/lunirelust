use crate::entities::{genre, record_genre};
use crate::{
    domains::luna::{
        domain::{Genre, GenreRepository},
        dto::{
            CreateGenreDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchGenreDto,
            UpdateGenreDto,
        },
    },
    entities::{GenreEntity, RecordGenreEntity},
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, PaginatorTrait as _, QueryFilter as _, Set,
};

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
        let name = genre.name;
        let link = genre.link.unwrap_or_default();
        let manual = genre.manual.unwrap_or(false);

        // Check if a genre with identical fields already exists
        let existing_genre = GenreEntity::find()
            .filter(genre::Column::Name.eq(&name))
            .filter(genre::Column::Link.eq(&link))
            .filter(genre::Column::Manual.eq(manual))
            .one(txn)
            .await?;

        if let Some(existing) = existing_genre {
            // Return existing genre's ID
            return Ok(existing.id);
        }

        // Create new genre if none exists
        let genre_active_model = genre::ActiveModel {
            name: Set(name),
            link: Set(link),
            manual: Set(manual),
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
            // Calculate the new values after update
            let new_name = genre.name.clone().unwrap_or(existing.name.clone());
            let new_link = genre.link.clone().unwrap_or(existing.link.clone());
            let new_manual = genre.manual.unwrap_or(existing.manual);

            // Check if a genre with the updated fields already exists (excluding current record)
            let matching_genre = GenreEntity::find()
                .filter(genre::Column::Name.eq(&new_name))
                .filter(genre::Column::Link.eq(&new_link))
                .filter(genre::Column::Manual.eq(new_manual))
                .filter(genre::Column::Id.ne(id))
                .one(txn)
                .await?;

            if let Some(matching) = matching_genre {
                // Delete the current genre and return the matching one
                GenreEntity::delete_by_id(id).exec(txn).await?;
                return Ok(Some(Genre::from(matching)));
            }

            // No matching genre found, proceed with update
            let mut genre_active_model: genre::ActiveModel = existing.into();

            if let Some(name) = genre.name {
                genre_active_model.name = Set(name);
            }

            if let Some(link) = genre.link {
                genre_active_model.link = Set(link);
            }

            if let Some(manual) = genre.manual {
                genre_active_model.manual = Set(manual);
            }

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
