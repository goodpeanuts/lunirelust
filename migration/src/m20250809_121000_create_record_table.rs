use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create record table
        manager
            .create_table(
                Table::create()
                    .table(Record::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Record::Id)
                            .string_len(255)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Record::Title)
                            .string_len(1024)
                            .not_null()
                            .default("Untitled"),
                    )
                    .col(
                        ColumnDef::new(Record::Date)
                            .date()
                            .not_null()
                            .default("1970-01-01"),
                    )
                    .col(
                        ColumnDef::new(Record::Duration)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Record::DirectorId)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Record::StudioId)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Record::LabelId)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Record::SeriesId)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Record::HasLinks)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Record::Permission)
                            .integer()
                            .not_null()
                            .default(3),
                    )
                    .col(
                        ColumnDef::new(Record::LocalImgCount)
                            .integer()
                            .not_null()
                            .default(-1),
                    )
                    .col(
                        ColumnDef::new(Record::CreateTime)
                            .date()
                            .not_null()
                            .default(Expr::current_date()),
                    )
                    .col(
                        ColumnDef::new(Record::UpdateTime)
                            .date()
                            .not_null()
                            .default(Expr::current_date()),
                    )
                    .col(
                        ColumnDef::new(Record::Creator)
                            .string_len(255)
                            .not_null()
                            .default("admin"),
                    )
                    .col(
                        ColumnDef::new(Record::ModifiedBy)
                            .string_len(255)
                            .not_null()
                            .default("admin"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_director")
                            .from(Record::Table, Record::DirectorId)
                            .to(Director::Table, Director::Id)
                            .on_delete(ForeignKeyAction::SetDefault),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_studio")
                            .from(Record::Table, Record::StudioId)
                            .to(Studio::Table, Studio::Id)
                            .on_delete(ForeignKeyAction::SetDefault),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_label")
                            .from(Record::Table, Record::LabelId)
                            .to(Label::Table, Label::Id)
                            .on_delete(ForeignKeyAction::SetDefault),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_series")
                            .from(Record::Table, Record::SeriesId)
                            .to(Series::Table, Series::Id)
                            .on_delete(ForeignKeyAction::SetDefault),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Record::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Record {
    Table,
    Id,
    Title,
    Date,
    Duration,
    DirectorId,
    StudioId,
    LabelId,
    SeriesId,
    HasLinks,
    Permission,
    LocalImgCount,
    CreateTime,
    UpdateTime,
    Creator,
    ModifiedBy,
}

// Reference to the basic tables
#[derive(DeriveIden)]
enum Director {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Studio {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Label {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Series {
    Table,
    Id,
}
