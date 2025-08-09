use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create links table
        manager
            .create_table(
                Table::create()
                    .table(Links::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Links::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Links::RecordId).string_len(255).not_null())
                    .col(
                        ColumnDef::new(Links::Name)
                            .string_len(255)
                            .not_null()
                            .default("None"),
                    )
                    .col(
                        ColumnDef::new(Links::Size)
                            .decimal_len(6, 2)
                            .not_null()
                            .default(-1.0),
                    )
                    .col(
                        ColumnDef::new(Links::Date)
                            .date()
                            .not_null()
                            .default("1970-01-01"),
                    )
                    .col(ColumnDef::new(Links::Link).text().not_null())
                    .col(
                        ColumnDef::new(Links::Star)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_links_record")
                            .from(Links::Table, Links::RecordId)
                            .to(Record::Table, Record::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Links::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Links {
    Table,
    Id,
    RecordId,
    Name,
    Size,
    Date,
    Link,
    Star,
}

// Reference to other tables
#[derive(DeriveIden)]
enum Record {
    Table,
    Id,
}
