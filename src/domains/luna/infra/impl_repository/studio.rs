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
        let name = create_dto.name;
        let link = create_dto.link.unwrap_or_default();
        let manual = create_dto.manual.unwrap_or(false);

        // Check if a studio with identical fields already exists
        let existing_studio = StudioEntity::find()
            .filter(studio::Column::Name.eq(&name))
            .filter(studio::Column::Link.eq(&link))
            .filter(studio::Column::Manual.eq(manual))
            .one(txn)
            .await?;

        if let Some(existing) = existing_studio {
            // Return existing studio's ID
            return Ok(existing.id);
        }

        // Create new studio if none exists
        let studio = studio::ActiveModel {
            name: Set(name),
            link: Set(link),
            manual: Set(manual),
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
        let existing_studio = StudioEntity::find_by_id(id).one(txn).await?;

        if let Some(existing) = existing_studio {
            // Calculate the new values after update
            let new_name = update_dto.name.clone().unwrap_or(existing.name.clone());
            let new_link = update_dto.link.clone().unwrap_or(existing.link.clone());
            let new_manual = update_dto.manual.unwrap_or(existing.manual);

            // Check if a studio with the updated fields already exists (excluding current record)
            let matching_studio = StudioEntity::find()
                .filter(studio::Column::Name.eq(&new_name))
                .filter(studio::Column::Link.eq(&new_link))
                .filter(studio::Column::Manual.eq(new_manual))
                .filter(studio::Column::Id.ne(id))
                .one(txn)
                .await?;

            if let Some(matching) = matching_studio {
                // Delete the current studio and return the matching one
                StudioEntity::delete_by_id(id).exec(txn).await?;
                return Ok(Some(Studio::from(matching)));
            }

            // No matching studio found, proceed with update
            let mut studio_active_model: studio::ActiveModel = existing.into();

            if let Some(name) = update_dto.name {
                studio_active_model.name = Set(name);
            }

            if let Some(link) = update_dto.link {
                studio_active_model.link = Set(link);
            }

            if let Some(manual) = update_dto.manual {
                studio_active_model.manual = Set(manual);
            }

            let updated_studio = studio_active_model.update(txn).await?;
            return Ok(Some(Studio::from(updated_studio)));
        }

        Ok(None)
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
