//! Add reverse lookup indexes used by the record detail loader.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();
        connection
            .execute_unprepared(
                "CREATE INDEX idx_idol_participation_record_id \
                 ON idol_participation (record_id)",
            )
            .await?;
        connection
            .execute_unprepared("CREATE INDEX idx_links_record_id ON links (record_id)")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();
        connection
            .execute_unprepared("DROP INDEX IF EXISTS idx_links_record_id")
            .await?;
        connection
            .execute_unprepared("DROP INDEX IF EXISTS idx_idol_participation_record_id")
            .await?;
        Ok(())
    }
}
