//! Migration: create user_record_interaction table.
//!
//! Stores per-user liked and viewed tracking for records.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserRecordInteraction::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRecordInteraction::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserRecordInteraction::UserId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRecordInteraction::RecordId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRecordInteraction::Liked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserRecordInteraction::Viewed)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserRecordInteraction::LikedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserRecordInteraction::ViewedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserRecordInteraction::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_record_interaction_user_id")
                            .from(UserRecordInteraction::Table, UserRecordInteraction::UserId)
                            .to(Alias::new("users"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_record_interaction_record_id")
                            .from(
                                UserRecordInteraction::Table,
                                UserRecordInteraction::RecordId,
                            )
                            .to(Alias::new("record"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_record_interaction_unique")
                    .table(UserRecordInteraction::Table)
                    .col(UserRecordInteraction::UserId)
                    .col(UserRecordInteraction::RecordId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_record_interaction_user_liked")
                    .table(UserRecordInteraction::Table)
                    .col(UserRecordInteraction::UserId)
                    .col(UserRecordInteraction::Liked)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserRecordInteraction::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserRecordInteraction {
    Table,
    Id,
    UserId,
    RecordId,
    Liked,
    Viewed,
    LikedAt,
    ViewedAt,
    CreatedAt,
}
