//! Migration: add composite index for viewed-record queries.
//!
//! Adds `idx_user_record_interaction_user_viewed_viewed_at` on
//! `(user_id, viewed, viewed_at)` to support `viewed_only` filtering
//! and `find_viewed_record_ids_paginated` without full table scans.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_user_record_interaction_user_viewed_viewed_at")
                    .table(Alias::new("user_record_interaction"))
                    .col(Alias::new("user_id"))
                    .col(Alias::new("viewed"))
                    .col(Alias::new("viewed_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_user_record_interaction_user_viewed_viewed_at")
                    .table(Alias::new("user_record_interaction"))
                    .to_owned(),
            )
            .await
    }
}
