//! Support literal substring search and relationship-based catalog matching.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const INDEXES: &[(&str, &str)] = &[
    ("idx_record_id_trgm", "record USING gin (id gin_trgm_ops)"),
    (
        "idx_record_title_trgm",
        "record USING gin (title gin_trgm_ops)",
    ),
    ("idx_record_director_id", "record (director_id)"),
    ("idx_record_studio_id", "record (studio_id)"),
    ("idx_record_label_id", "record (label_id)"),
    ("idx_record_series_id", "record (series_id)"),
    (
        "idx_record_genre_genre_record",
        "record_genre (genre_id, record_id)",
    ),
    (
        "idx_director_name_trgm",
        "director USING gin (name gin_trgm_ops)",
    ),
    (
        "idx_studio_name_trgm",
        "studio USING gin (name gin_trgm_ops)",
    ),
    ("idx_label_name_trgm", "label USING gin (name gin_trgm_ops)"),
    (
        "idx_series_name_trgm",
        "series USING gin (name gin_trgm_ops)",
    ),
    ("idx_genre_name_trgm", "genre USING gin (name gin_trgm_ops)"),
    ("idx_idol_name_trgm", "idol USING gin (name gin_trgm_ops)"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();
        connection
            .execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm")
            .await?;
        for (name, definition) in INDEXES {
            connection
                .execute_unprepared(&format!("CREATE INDEX {name} ON {definition}"))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (name, _) in INDEXES.iter().rev() {
            manager
                .get_connection()
                .execute_unprepared(&format!("DROP INDEX {name}"))
                .await?;
        }
        // The extension may be shared by other application indexes.
        Ok(())
    }
}
