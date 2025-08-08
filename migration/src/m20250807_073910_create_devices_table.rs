use sea_orm_migration::prelude::*;

// Foreign key references for the table
use super::m20250807_073903_create_users_table::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Devices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Devices::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Devices::UserId).string_len(36).not_null())
                    .col(ColumnDef::new(Devices::Name).string_len(128).not_null())
                    .col(ColumnDef::new(Devices::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Devices::DeviceOs).string_len(16).not_null())
                    .col(
                        ColumnDef::new(Devices::RegisteredAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Devices::CreatedBy).string_len(36).null())
                    .col(
                        ColumnDef::new(Devices::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Devices::ModifiedBy).string_len(36).null())
                    .col(
                        ColumnDef::new(Devices::ModifiedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_devices_user_id")
                            .from(Devices::Table, Devices::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_devices_user_id")
                    .table(Devices::Table)
                    .col(Devices::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_devices_status")
                    .table(Devices::Table)
                    .col(Devices::Status)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Devices::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Devices {
    Table,
    Id,
    UserId,
    Name,
    Status,
    DeviceOs,
    RegisteredAt,
    CreatedBy,
    CreatedAt,
    ModifiedBy,
    ModifiedAt,
}
