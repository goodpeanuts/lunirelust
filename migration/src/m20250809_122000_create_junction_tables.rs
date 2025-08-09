use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create record_genre junction table
        manager
            .create_table(
                Table::create()
                    .table(RecordGenre::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecordGenre::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RecordGenre::RecordId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecordGenre::GenreId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecordGenre::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_genre_record")
                            .from(RecordGenre::Table, RecordGenre::RecordId)
                            .to(Record::Table, Record::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_genre_genre")
                            .from(RecordGenre::Table, RecordGenre::GenreId)
                            .to(Genre::Table, Genre::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for record_id and genre_id combination
        manager
            .create_index(
                Index::create()
                    .name("idx_record_genre_unique")
                    .table(RecordGenre::Table)
                    .col(RecordGenre::RecordId)
                    .col(RecordGenre::GenreId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create idol_participation junction table
        manager
            .create_table(
                Table::create()
                    .table(IdolParticipation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(IdolParticipation::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(IdolParticipation::IdolId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IdolParticipation::RecordId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IdolParticipation::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_idol_participation_idol")
                            .from(IdolParticipation::Table, IdolParticipation::IdolId)
                            .to(Idol::Table, Idol::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_idol_participation_record")
                            .from(IdolParticipation::Table, IdolParticipation::RecordId)
                            .to(Record::Table, Record::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for idol_id and record_id combination
        manager
            .create_index(
                Index::create()
                    .name("idx_idol_participation_unique")
                    .table(IdolParticipation::Table)
                    .col(IdolParticipation::IdolId)
                    .col(IdolParticipation::RecordId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecordGenre::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(IdolParticipation::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RecordGenre {
    Table,
    Id,
    RecordId,
    GenreId,
    Manual,
}

#[derive(DeriveIden)]
enum IdolParticipation {
    Table,
    Id,
    IdolId,
    RecordId,
    Manual,
}

// Reference to other tables
#[derive(DeriveIden)]
enum Record {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Genre {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Idol {
    Table,
    Id,
}
