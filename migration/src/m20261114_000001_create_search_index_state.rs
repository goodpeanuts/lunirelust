//! Track whether an optional search index needs rebuilding after PostgreSQL-only use.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE search_index_state (
                id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
                generation BIGINT NOT NULL DEFAULT 0,
                indexed_generation BIGINT NOT NULL DEFAULT -1
            );
            INSERT INTO search_index_state (id, generation, indexed_generation)
            VALUES (TRUE, 0, -1)",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE search_index_state")
            .await?;
        Ok(())
    }
}
