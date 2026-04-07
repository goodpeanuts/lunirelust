use crate::{
    common::error::AppError,
    domains::device::{
        domain::{repository::DeviceRepository, service::DeviceServiceTrait},
        dto::device_dto::{CreateDeviceDto, DeviceDto, UpdateDeviceDto, UpdateManyDevicesDto},
        infra::impl_repository::DeviceRepo,
    },
};

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling device-related operations
/// such as creating, updating, deleting, and fetching devices.
/// It uses a repository pattern to abstract the data access layer.
#[derive(Clone)]
pub struct DeviceService {
    db: DatabaseConnection,
    repo: Arc<dyn DeviceRepository + Send + Sync>,
}

#[async_trait]
impl DeviceServiceTrait for DeviceService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn DeviceServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(DeviceRepo {}),
        })
    }

    async fn get_device_by_id(&self, id: String) -> Result<DeviceDto, AppError> {
        self.repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(DeviceDto::from)
            .ok_or_else(|| AppError::NotFound("Device not found".into()))
    }

    async fn get_devices(&self) -> Result<Vec<DeviceDto>, AppError> {
        let devices = self.repo.find_all(&self.db).await?;
        Ok(devices.into_iter().map(Into::into).collect())
    }

    async fn create_device(&self, payload: CreateDeviceDto) -> Result<DeviceDto, AppError> {
        let tx = self.db.begin().await?;
        let device = match self.repo.create(&tx, payload).await {
            Ok(d) => d,
            Err(e) => {
                tx.rollback().await.ok();
                return Err(AppError::DatabaseError(e));
            }
        };
        tx.commit().await?;
        Ok(DeviceDto::from(device))
    }

    async fn update_device(
        &self,
        id: String,
        payload: UpdateDeviceDto,
    ) -> Result<DeviceDto, AppError> {
        let tx = self.db.begin().await?;
        match self.repo.update(&tx, id, payload).await {
            Ok(Some(device)) => {
                tx.commit().await?;
                Ok(DeviceDto::from(device))
            }
            Ok(None) => {
                tx.rollback().await?;
                Err(AppError::NotFound("Device not found".into()))
            }
            Err(e) => {
                tx.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn delete_device(&self, id: String) -> Result<String, AppError> {
        let tx = self.db.begin().await?;
        match self.repo.delete(&tx, id).await {
            Ok(true) => {
                tx.commit().await?;
                Ok("Device deleted".into())
            }
            Ok(false) => {
                tx.rollback().await?;
                Err(AppError::NotFound("Device not found".into()))
            }
            Err(e) => {
                tx.rollback().await.ok();
                Err(AppError::DatabaseError(e))
            }
        }
    }

    async fn update_many_devices(
        &self,
        user_id: String,
        modified_by: String,
        payload: UpdateManyDevicesDto,
    ) -> Result<String, AppError> {
        let tx = self.db.begin().await?;
        if let Err(e) = self
            .repo
            .update_many(&tx, user_id, modified_by, payload)
            .await
        {
            tx.rollback().await.ok();
            return Err(AppError::DatabaseError(e));
        }
        tx.commit().await?;
        Ok("Devices updated".into())
    }
}
