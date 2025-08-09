use sea_orm_migration::prelude::*;

use crate::m20250807_073903_create_users_table::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UploadedFiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UploadedFiles::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::UserId)
                            .string_len(36)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::FileName)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::OriginFileName)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::FileRelativePath)
                            .string_len(256)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::FileUrl)
                            .string_len(256)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::ContentType)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::FileSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::FileType)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::CreatedBy)
                            .string_len(36)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::ModifiedBy)
                            .string_len(36)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UploadedFiles::ModifiedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_uploaded_files_user_id")
                            .from(UploadedFiles::Table, UploadedFiles::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UploadedFiles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UploadedFiles {
    Table,
    Id,
    UserId,
    FileName,
    OriginFileName,
    FileRelativePath,
    FileUrl,
    ContentType,
    FileSize,
    FileType,
    CreatedBy,
    CreatedAt,
    ModifiedBy,
    ModifiedAt,
}
