use crate::entities::{record, studio};
use crate::{
    domains::luna::{
        domain::{Studio, StudioRepository},
        dto::{CreateStudioDto, EntityCountDto, SearchStudioDto, UpdateStudioDto},
    },
    entities::{RecordEntity, StudioEntity},
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, PaginatorTrait as _, QueryFilter as _, Set,
};

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
