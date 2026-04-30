//! Migration: create crawl_task, crawl_code_result, crawl_page_result tables.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // crawl_task table
        manager
            .create_table(
                Table::create()
                    .table(CrawlTask::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrawlTask::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CrawlTask::TaskType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(CrawlTask::Status).string_len(32).not_null())
                    .col(ColumnDef::new(CrawlTask::UserId).string_len(255).not_null())
                    .col(
                        ColumnDef::new(CrawlTask::MarkLiked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(CrawlTask::MarkViewed)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(CrawlTask::InputPayload).text().null())
                    .col(ColumnDef::new(CrawlTask::MaxPages).integer().null())
                    .col(
                        ColumnDef::new(CrawlTask::TotalCodes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CrawlTask::SuccessCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CrawlTask::FailCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CrawlTask::SkipCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(CrawlTask::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(CrawlTask::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CrawlTask::StartedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CrawlTask::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crawl_task_user_id")
                            .from(CrawlTask::Table, CrawlTask::UserId)
                            .to(Alias::new("users"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_crawl_task_user_id")
                    .table(CrawlTask::Table)
                    .col(CrawlTask::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_crawl_task_status")
                    .table(CrawlTask::Table)
                    .col(CrawlTask::Status)
                    .to_owned(),
            )
            .await?;

        // crawl_code_result table
        manager
            .create_table(
                Table::create()
                    .table(CrawlCodeResult::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrawlCodeResult::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CrawlCodeResult::TaskId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlCodeResult::Code)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlCodeResult::Status)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlCodeResult::RecordId)
                            .string_len(255)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CrawlCodeResult::ImagesDownloaded)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(CrawlCodeResult::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(CrawlCodeResult::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crawl_code_result_task_id")
                            .from(CrawlCodeResult::Table, CrawlCodeResult::TaskId)
                            .to(CrawlTask::Table, CrawlTask::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_crawl_code_result_task_id")
                    .table(CrawlCodeResult::Table)
                    .col(CrawlCodeResult::TaskId)
                    .to_owned(),
            )
            .await?;

        // crawl_page_result table
        manager
            .create_table(
                Table::create()
                    .table(CrawlPageResult::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CrawlPageResult::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CrawlPageResult::TaskId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlPageResult::PageNumber)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlPageResult::Status)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CrawlPageResult::RecordsFound)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CrawlPageResult::RecordsCrawled)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(CrawlPageResult::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(CrawlPageResult::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_crawl_page_result_task_id")
                            .from(CrawlPageResult::Table, CrawlPageResult::TaskId)
                            .to(CrawlTask::Table, CrawlTask::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_crawl_page_result_task_id")
                    .table(CrawlPageResult::Table)
                    .col(CrawlPageResult::TaskId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CrawlPageResult::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrawlCodeResult::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CrawlTask::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CrawlTask {
    Table,
    Id,
    TaskType,
    Status,
    UserId,
    MarkLiked,
    MarkViewed,
    InputPayload,
    MaxPages,
    TotalCodes,
    SuccessCount,
    FailCount,
    SkipCount,
    ErrorMessage,
    CreatedAt,
    StartedAt,
    CompletedAt,
}

#[derive(DeriveIden)]
enum CrawlCodeResult {
    Table,
    Id,
    TaskId,
    Code,
    Status,
    RecordId,
    ImagesDownloaded,
    ErrorMessage,
    CreatedAt,
}

#[derive(DeriveIden)]
enum CrawlPageResult {
    Table,
    Id,
    TaskId,
    PageNumber,
    Status,
    RecordsFound,
    RecordsCrawled,
    ErrorMessage,
    CreatedAt,
}
