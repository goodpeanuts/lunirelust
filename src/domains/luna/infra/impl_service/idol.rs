use crate::domains::search::SearchEntityType;
use crate::{
    common::{config::Config, error::AppError},
    domains::luna::{
        domain::{IdolRepository, IdolServiceTrait},
        dto::{
            CreateIdolDto, EntityCountDto, IdolDto, IdolWithoutImageDto, PaginatedResponse,
            PaginationQuery, SearchIdolDto, UpdateIdolDto,
        },
        infra::{search_outbox, IdolRepo},
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

/// Service struct for handling idol-related operations.
#[derive(Clone)]
pub struct IdolService {
    db: DatabaseConnection,
    repo: Arc<dyn IdolRepository + Send + Sync>,
    config: Config,
}

#[async_trait]
impl IdolServiceTrait for IdolService {
    fn create_service(db: DatabaseConnection, config: Config) -> Arc<dyn IdolServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(IdolRepo),
            config,
        })
    }

    async fn get_idol_by_id(&self, id: i64) -> Result<IdolDto, AppError> {
        let idol = self
            .repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?;

        idol.map(IdolDto::from)
            .ok_or_else(|| AppError::NotFound("Idol not found".into()))
    }

    async fn get_idol_list(&self, search_dto: SearchIdolDto) -> Result<Vec<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(idols.into_iter().map(IdolDto::from).collect())
    }

    async fn get_idol_list_paginated(
        &self,
        search_dto: SearchIdolDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<IdolDto>, AppError> {
        let paginated = self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(PaginatedResponse {
            count: paginated.count,
            next: paginated.next,
            previous: paginated.previous,
            results: paginated.results.into_iter().map(IdolDto::from).collect(),
        })
    }

    async fn get_idols(&self) -> Result<Vec<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(idols.into_iter().map(IdolDto::from).collect())
    }

    async fn create_idol(&self, create_dto: CreateIdolDto) -> Result<IdolDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let entity_name = create_dto.name.clone();
        let (id, was_created) = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        if was_created {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Idol,
                id,
                &entity_name,
                Vec::new(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_idol_by_id(id).await
    }

    async fn update_idol(&self, id: i64, update_dto: UpdateIdolDto) -> Result<IdolDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let pre_affected =
            search_outbox::find_affected_record_ids(&txn, SearchEntityType::Idol, id)
                .await
                .map_err(AppError::DatabaseError)?;

        let updated_idol = self
            .repo
            .update(&txn, id, update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let Some(idol) = updated_idol else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::NotFound("Idol not found".into()));
        };

        let surviving_id = idol.id;
        if surviving_id != id {
            search_outbox::outbox_entity_delete(&txn, SearchEntityType::Idol, id, vec![])
                .await
                .map_err(AppError::DatabaseError)?;
            let mut all_affected = pre_affected.clone();
            let surviving_affected =
                search_outbox::find_affected_record_ids(&txn, SearchEntityType::Idol, surviving_id)
                    .await
                    .map_err(AppError::DatabaseError)?;
            all_affected.extend(surviving_affected);
            all_affected.sort_unstable();
            all_affected.dedup();
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Idol,
                surviving_id,
                &idol.name,
                all_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &all_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        } else {
            search_outbox::outbox_entity_upsert(
                &txn,
                SearchEntityType::Idol,
                id,
                &idol.name,
                pre_affected.clone(),
            )
            .await
            .map_err(AppError::DatabaseError)?;
            search_outbox::outbox_fanout_records(&txn, &pre_affected)
                .await
                .map_err(AppError::DatabaseError)?;
        }

        txn.commit().await.map_err(AppError::DatabaseError)?;

        Ok(IdolDto::from(idol))
    }

    async fn delete_idol(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let affected = search_outbox::find_affected_record_ids(&txn, SearchEntityType::Idol, id)
            .await
            .map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id)
            .await
            .map_err(AppError::DatabaseError)?;

        if !deleted {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::NotFound("Idol not found".into()));
        }

        search_outbox::outbox_entity_delete(&txn, SearchEntityType::Idol, id, affected.clone())
            .await
            .map_err(AppError::DatabaseError)?;
        search_outbox::outbox_fanout_records(&txn, &affected)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;
        Ok("Idol deleted successfully".to_owned())
    }

    /// Gets record counts grouped by idols.
    async fn get_idol_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_idol_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }

    /// Gets idols that don't have any images in the media directory.
    async fn get_idols_without_images(
        &self,
        assets_private_path: &str,
    ) -> Result<Vec<IdolWithoutImageDto>, AppError> {
        // Get all idols from database
        let all_idols = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        let mut idols_without_images = Vec::new();

        for idol in all_idols {
            if idol.id <= 0 {
                continue; // Skip invalid idols
            }

            // Check if the media directory exists and has images
            let idol_media_dir = Path::new(assets_private_path)
                .join("images")
                .join("idol")
                .join(&idol.name);

            let has_images = if idol_media_dir.exists() {
                // Check if directory has any image files
                match fs::read_dir(&idol_media_dir).await {
                    Ok(mut entries) => {
                        let mut has_any_images = false;
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            if let Some(file_name) = entry.file_name().to_str() {
                                let file_path = entry.path();
                                if file_path.is_file() {
                                    // Check if it's an image file
                                    let extension = file_name
                                        .split('.')
                                        .next_back()
                                        .unwrap_or("")
                                        .to_lowercase();
                                    if self.config.asset_allowed_extensions.contains(&extension) {
                                        has_any_images = true;
                                        break;
                                    }
                                }
                            }
                        }
                        has_any_images
                    }
                    Err(_) => false,
                }
            } else {
                false
            };

            if !has_images {
                idols_without_images.push(IdolWithoutImageDto {
                    id: idol.id,
                    name: idol.name,
                    link: idol.link,
                });
            }
        }

        Ok(idols_without_images)
    }
}
