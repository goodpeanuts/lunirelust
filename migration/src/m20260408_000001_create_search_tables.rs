//! Migration: create search sync infrastructure tables.
//!
//! Creates two tables:
//! - `search_sync_events`: outbox table for durable, transactional search index updates.
//! - `search_document_versions`: tombstone table for version tracking and stale-event detection.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Outbox table for durable search sync events
        manager
            .create_table(
                Table::create()
                    .table(SearchSyncEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SearchSyncEvents::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::EntityType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::EntityId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::EventType)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::EntityVersion)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(SearchSyncEvents::Payload).json().null())
                    .col(
                        ColumnDef::new(SearchSyncEvents::AffectedRecordIds)
                            .json()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::ProcessedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::ClaimedBy)
                            .string_len(64)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SearchSyncEvents::ClaimedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for polling pending events
        manager
            .create_index(
                Index::create()
                    .name("idx_search_sync_events_pending")
                    .table(SearchSyncEvents::Table)
                    .col(SearchSyncEvents::ProcessedAt)
                    .col(SearchSyncEvents::ClaimedBy)
                    .to_owned(),
            )
            .await?;

        // Index for reclaiming expired claims
        manager
            .create_index(
                Index::create()
                    .name("idx_search_sync_events_claimed")
                    .table(SearchSyncEvents::Table)
                    .col(SearchSyncEvents::ClaimedAt)
                    .to_owned(),
            )
            .await?;

        // Tombstone table for version tracking and delete-safe staleness detection
        manager
            .create_table(
                Table::create()
                    .table(SearchDocumentVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SearchDocumentVersions::EntityType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SearchDocumentVersions::EntityId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SearchDocumentVersions::LastVersion)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SearchDocumentVersions::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(SearchDocumentVersions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .primary_key(
                        Index::create()
                            .col(SearchDocumentVersions::EntityType)
                            .col(SearchDocumentVersions::EntityId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SearchDocumentVersions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(SearchSyncEvents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SearchSyncEvents {
    Table,
    Id,
    EntityType,
    EntityId,
    EventType,
    EntityVersion,
    Payload,
    AffectedRecordIds,
    CreatedAt,
    ProcessedAt,
    ClaimedBy,
    ClaimedAt,
}

#[derive(DeriveIden)]
enum SearchDocumentVersions {
    Table,
    EntityType,
    EntityId,
    LastVersion,
    IsDeleted,
    UpdatedAt,
}
