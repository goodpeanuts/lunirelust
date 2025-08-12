use crate::entities::{idol, idol_participation};
use crate::{
    domains::luna::{
        domain::{Idol, IdolRepository},
        dto::{CreateIdolDto, EntityCountDto, SearchIdolDto, UpdateIdolDto},
    },
    entities::{IdolEntity, IdolParticipationEntity},
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, PaginatorTrait as _, QueryFilter as _, Set,
};

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
        let name = idol.name;
        let link = idol.link.unwrap_or_default();
        let manual = idol.manual.unwrap_or(false);

        // Check if an idol with identical fields already exists
        let existing_idol = IdolEntity::find()
            .filter(idol::Column::Name.eq(&name))
            .filter(idol::Column::Link.eq(&link))
            .filter(idol::Column::Manual.eq(manual))
            .one(txn)
            .await?;

        if let Some(existing) = existing_idol {
            // Return existing idol's ID
            return Ok(existing.id);
        }

        // Create new idol if none exists
        let active_idol = idol::ActiveModel {
            name: Set(name),
            link: Set(link),
            manual: Set(manual),
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
                // Calculate the new values after update
                let new_name = idol.name.clone().unwrap_or(existing.name.clone());
                let new_link = idol.link.clone().unwrap_or(existing.link.clone());
                let new_manual = idol.manual.unwrap_or(existing.manual);

                // Check if an idol with the updated fields already exists (excluding current record)
                let matching_idol = IdolEntity::find()
                    .filter(idol::Column::Name.eq(&new_name))
                    .filter(idol::Column::Link.eq(&new_link))
                    .filter(idol::Column::Manual.eq(new_manual))
                    .filter(idol::Column::Id.ne(id))
                    .one(txn)
                    .await?;

                if let Some(matching) = matching_idol {
                    // Delete the current idol and return the matching one
                    IdolEntity::delete_by_id(id).exec(txn).await?;
                    return Ok(Some(Idol::from(matching)));
                }

                // No matching idol found, proceed with update
                let mut active_idol: idol::ActiveModel = existing.into();

                if let Some(name) = idol.name {
                    active_idol.name = Set(name);
                }
                if let Some(link) = idol.link {
                    active_idol.link = Set(link);
                }
                if let Some(manual) = idol.manual {
                    active_idol.manual = Set(manual);
                }

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
