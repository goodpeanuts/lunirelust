//! Migration: add composite B-tree index on record(date DESC, id ASC)
//!
//! Backs the deterministic newest-first ordering applied to all record
//! list queries with a stable tie-breaker on the primary key.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("CREATE INDEX idx_record_date_desc ON record (date DESC, id ASC)")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("DROP INDEX IF EXISTS idx_record_date_desc")
            .await?;
        Ok(())
    }
}
