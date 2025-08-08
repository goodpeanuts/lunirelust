use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait as _, Set,
};
use std::str::FromStr as _;
use uuid::Uuid;

use crate::domains::device::domain::model::{Device, DeviceOS, DeviceStatus};
use crate::domains::device::domain::repository::DeviceRepository;
use crate::domains::device::dto::device_dto::{
    CreateDeviceDto, UpdateDeviceDto, UpdateManyDevicesDto,
};
use crate::entities::devices;

pub struct DeviceRepo;

impl DeviceRepo {
    fn entity_to_model(entity: devices::Model) -> Result<Device, DbErr> {
        Ok(Device {
            id: entity.id,
            user_id: entity.user_id,
            name: entity.name,
            device_os: DeviceOS::from_str(&entity.device_os)
                .map_err(|e| DbErr::Type(e.to_string()))?,
            status: DeviceStatus::from_str(&entity.status)
                .map_err(|e| DbErr::Type(e.to_string()))?,
            registered_at: entity.registered_at,
            created_by: entity.created_by,
            created_at: entity.created_at,
            modified_by: entity.modified_by,
            modified_at: entity.modified_at,
        })
    }
}

#[async_trait]
impl DeviceRepository for DeviceRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Device>, DbErr> {
        let devices = devices::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .map(Self::entity_to_model)
            .collect::<Result<Vec<_>, DbErr>>()?;

        Ok(devices)
    }

    async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Option<Device>, DbErr> {
        let device = devices::Entity::find_by_id(id)
            .one(db)
            .await?
            .map(Self::entity_to_model)
            .transpose()?;

        Ok(device)
    }

    async fn create(
        &self,
        tx: &DatabaseTransaction,
        device: CreateDeviceDto,
    ) -> Result<Device, DbErr> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let active_device = devices::ActiveModel {
            id: Set(id.clone()),
            user_id: Set(device.user_id),
            name: Set(device.name),
            status: Set(device.status.to_string()),
            device_os: Set(device.device_os.to_string()),
            registered_at: Set(device.registered_at),
            created_by: Set(Some(device.modified_by.clone())),
            created_at: Set(Some(now)),
            modified_by: Set(Some(device.modified_by)),
            modified_at: Set(Some(now)),
        };

        let inserted = active_device.insert(tx).await?;
        Self::entity_to_model(inserted)
    }

    async fn update(
        &self,
        tx: &DatabaseTransaction,
        id: String,
        device: UpdateDeviceDto,
    ) -> Result<Option<Device>, DbErr> {
        let existing = devices::Entity::find_by_id(&id).one(tx).await?;

        if let Some(entity) = existing {
            let mut active_device: devices::ActiveModel = entity.into();

            if let Some(value) = device.user_id {
                active_device.user_id = Set(value);
            }
            if let Some(value) = device.name {
                active_device.name = Set(value);
            }
            if let Some(value) = device.status {
                active_device.status = Set(value.to_string());
            }
            if let Some(value) = device.device_os {
                active_device.device_os = Set(value.to_string());
            }
            if let Some(value) = device.registered_at {
                active_device.registered_at = Set(Some(value));
            }

            active_device.modified_by = Set(Some(device.modified_by));
            active_device.modified_at = Set(Some(chrono::Utc::now()));

            let updated = active_device.update(tx).await?;
            return Ok(Some(Self::entity_to_model(updated)?));
        }

        Ok(None)
    }

    async fn update_many(
        &self,
        tx: &DatabaseTransaction,
        user_id: String,
        modified_by: String,
        update_devices: UpdateManyDevicesDto,
    ) -> Result<(), DbErr> {
        let now = chrono::Utc::now();

        let active_devices: Vec<devices::ActiveModel> = update_devices
            .devices
            .into_iter()
            .map(|device| {
                let device_id = device.id.unwrap_or_else(|| Uuid::new_v4().to_string());
                devices::ActiveModel {
                    id: Set(device_id),
                    user_id: Set(user_id.clone()),
                    name: Set(device.name),
                    status: Set(device.status.to_string()),
                    device_os: Set(device.device_os.to_string()),
                    registered_at: Set(Some(now)),
                    created_by: Set(Some(modified_by.clone())),
                    created_at: Set(Some(now)),
                    modified_by: Set(Some(modified_by.clone())),
                    modified_at: Set(Some(now)),
                }
            })
            .collect();

        // Use batch insert with upsert (on_conflict)
        devices::Entity::insert_many(active_devices)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(devices::Column::Id)
                    .update_columns([
                        devices::Column::Name,
                        devices::Column::Status,
                        devices::Column::DeviceOs,
                        devices::Column::ModifiedBy,
                        devices::Column::ModifiedAt,
                    ])
                    .to_owned(),
            )
            .exec(tx)
            .await?;

        Ok(())
    }

    async fn delete(&self, tx: &DatabaseTransaction, id: String) -> Result<bool, DbErr> {
        let result = devices::Entity::delete_by_id(id).exec(tx).await?;

        Ok(result.rows_affected > 0)
    }
}
