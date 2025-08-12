use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create director table
        manager
            .create_table(
                Table::create()
                    .table(Director::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Director::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Director::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Director::Link).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Director::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Create studio table
        manager
            .create_table(
                Table::create()
                    .table(Studio::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Studio::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Studio::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Studio::Link).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Studio::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Create label table
        manager
            .create_table(
                Table::create()
                    .table(Label::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Label::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Label::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Label::Link).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Label::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Create series table
        manager
            .create_table(
                Table::create()
                    .table(Series::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Series::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Series::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Series::Link).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Series::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Create genre table
        manager
            .create_table(
                Table::create()
                    .table(Genre::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Genre::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Genre::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Genre::Link).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Genre::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Create idol table
        manager
            .create_table(
                Table::create()
                    .table(Idol::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Idol::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Idol::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Idol::Link).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Idol::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Director::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Studio::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Label::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Series::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Genre::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Idol::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Director {
    Table,
    Id,
    Name,
    Link,
    Manual,
}

#[derive(DeriveIden)]
enum Studio {
    Table,
    Id,
    Name,
    Link,
    Manual,
}

#[derive(DeriveIden)]
enum Label {
    Table,
    Id,
    Name,
    Link,
    Manual,
}

#[derive(DeriveIden)]
enum Series {
    Table,
    Id,
    Name,
    Link,
    Manual,
}

#[derive(DeriveIden)]
enum Genre {
    Table,
    Id,
    Name,
    Link,
    Manual,
}

#[derive(DeriveIden)]
enum Idol {
    Table,
    Id,
    Name,
    Link,
    Manual,
}
