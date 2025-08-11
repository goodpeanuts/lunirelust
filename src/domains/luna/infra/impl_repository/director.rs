use crate::entities::{director, record};
use crate::{
    domains::luna::{
        domain::{Director, DirectorRepository},
        dto::{
            CreateDirectorDto, EntityCountDto, PaginatedResponse, PaginationQuery,
            SearchDirectorDto, UpdateDirectorDto,
        },
    },
    entities::{DirectorEntity, RecordEntity},
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
