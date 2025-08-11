use crate::entities::{label, record};
use crate::{
    domains::luna::{
        domain::{Label, LabelRepository},
        dto::{
            CreateLabelDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchLabelDto,
            UpdateLabelDto,
        },
    },
    entities::{LabelEntity, RecordEntity},
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, PaginatorTrait as _, QueryFilter as _, Set,
};

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
