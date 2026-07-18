//! Migration: create crawl_entity_progress table.
//!
//! Tracks per-entity crawl state for the entity-auto-crawl task type. This
//! table is round-only: it stores `last_crawled_round` (the rotation counter
//! driving round-robin selection) and a pointer to the latest task. Success or
//! failure state is NOT stored here -- it is read from `crawl_task` via the
//! `last_task_id` join. There is deliberately no `status` and no `last_page`
//! column (crawls always restart from page 1).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CrawlEntityProgress::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrawlEntityProgress::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::EntityType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::EntityId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::EntityName)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::LastCrawledRound)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::LastTaskId)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::LastCrawledAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CrawlEntityProgress::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // UNIQUE(entity_type, entity_id): one progress row per entity per type.
        manager
            .create_index(
                Index::create()
                    .name("idx_crawl_entity_progress_unique")
                    .table(CrawlEntityProgress::Table)
                    .col(CrawlEntityProgress::EntityType)
                    .col(CrawlEntityProgress::EntityId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // (entity_type, last_crawled_round): supports the per-type MIN scan that
        // derives `current_round`, and round-restricted selection.
        manager
            .create_index(
                Index::create()
                    .name("idx_crawl_entity_progress_type_round")
                    .table(CrawlEntityProgress::Table)
                    .col(CrawlEntityProgress::EntityType)
                    .col(CrawlEntityProgress::LastCrawledRound)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrawlEntityProgress::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CrawlEntityProgress {
    Table,
    Id,
    EntityType,
    EntityId,
    EntityName,
    LastCrawledRound,
    LastTaskId,
    LastCrawledAt,
    CreatedAt,
    UpdatedAt,
}
