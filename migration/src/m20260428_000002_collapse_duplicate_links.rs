//! Migration: create links_duplicate_archive table, collapse duplicate links,
//! and add unique index on links(record_id, link).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Create links_duplicate_archive table
        manager
            .create_table(
                Table::create()
                    .table(LinksDuplicateArchive::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::RecordId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::Name)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::Size)
                            .decimal()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::Date)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::Link)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::Star)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(LinksDuplicateArchive::RetainedLinkId)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Step 2: Collapse duplicates using raw SQL
        // Find all duplicate groups, merge fields, archive, and delete
        let conn = manager.get_connection();

        // Archive and collapse duplicates
        // For each group of (record_id, link) with count > 1:
        // 1. Keep the row with lowest id as base
        // 2. Merge star with OR semantics
        // 3. Merge name/size/date taking non-default values (first-writer priority)
        // 4. Archive non-base rows to links_duplicate_archive
        // 5. Delete non-base rows

        let collapse_sql = r#"
        DO $$
        DECLARE
            dup_record_id TEXT;
            dup_link TEXT;
            base_id BIGINT;
            merged_star BOOLEAN;
            merged_name TEXT;
            merged_size DECIMAL;
            merged_date DATE;
            dup_row RECORD;
        BEGIN
            FOR dup_record_id, dup_link IN
                SELECT l.record_id, l.link
                FROM links l
                WHERE l.link != '' AND TRIM(l.link) != ''
                GROUP BY l.record_id, l.link
                HAVING COUNT(*) > 1
            LOOP
                -- Find the base row (lowest id)
                SELECT id INTO base_id
                FROM links
                WHERE record_id = dup_record_id AND link = dup_link
                ORDER BY id ASC
                LIMIT 1;

                -- Compute merged values
                merged_star := FALSE;
                merged_name := 'None';
                merged_size := -1;
                merged_date := '1970-01-01'::DATE;

                FOR dup_row IN
                    SELECT id, star, name, size, date
                    FROM links
                    WHERE record_id = dup_record_id AND link = dup_link
                    ORDER BY id ASC
                LOOP
                    -- Star: OR semantics
                    IF dup_row.star THEN
                        merged_star := TRUE;
                    END IF;

                    -- Name: first non-default wins (first-writer priority)
                    IF merged_name = 'None' AND dup_row.name != 'None' THEN
                        merged_name := dup_row.name;
                    END IF;

                    -- Size: first non-default wins
                    IF merged_size = -1 AND dup_row.size != -1 THEN
                        merged_size := dup_row.size;
                    END IF;

                    -- Date: first non-default wins
                    IF merged_date = '1970-01-01'::DATE AND dup_row.date != '1970-01-01'::DATE THEN
                        merged_date := dup_row.date;
                    END IF;
                END LOOP;

                -- Archive non-base rows
                INSERT INTO links_duplicate_archive (record_id, name, size, date, link, star, retained_link_id)
                SELECT record_id, name, size, date, link, star, base_id
                FROM links
                WHERE record_id = dup_record_id AND link = dup_link AND id != base_id;

                -- Delete non-base rows
                DELETE FROM links
                WHERE record_id = dup_record_id AND link = dup_link AND id != base_id;

                -- Update base row with merged values
                UPDATE links
                SET star = merged_star,
                    name = merged_name,
                    size = merged_size,
                    date = merged_date
                WHERE id = base_id;
            END LOOP;
        END;
        $$;
        "#;

        conn.execute_unprepared(collapse_sql).await?;

        // Step 3: Create unique index on (record_id, link) for non-empty links
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_links_record_id_link_unique ON links (record_id, link) WHERE link != '' AND TRIM(link) != '';"
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared("DROP INDEX IF EXISTS idx_links_record_id_link_unique;")
            .await?;

        manager
            .drop_table(Table::drop().table(LinksDuplicateArchive::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LinksDuplicateArchive {
    Table,
    Id,
    RecordId,
    Name,
    Size,
    Date,
    Link,
    Star,
    RetainedLinkId,
}
