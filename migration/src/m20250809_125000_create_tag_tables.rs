use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create tag_category table
        manager
            .create_table(
                Table::create()
                    .table(TagCategory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TagCategory::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TagCategory::Name).string_len(255).not_null())
                    .col(ColumnDef::new(TagCategory::Description).text())
                    .col(
                        ColumnDef::new(TagCategory::CreateTime)
                            .date()
                            .not_null()
                            .default(Expr::current_date()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create tag table
        manager
            .create_table(
                Table::create()
                    .table(Tag::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tag::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tag::CategoryId).big_integer().not_null())
                    .col(ColumnDef::new(Tag::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Tag::Description).text())
                    .col(
                        ColumnDef::new(Tag::CreateTime)
                            .date()
                            .not_null()
                            .default(Expr::current_date()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tag_category")
                            .from(Tag::Table, Tag::CategoryId)
                            .to(TagCategory::Table, TagCategory::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        // Create record_tag junction table
        manager
            .create_table(
                Table::create()
                    .table(RecordTag::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecordTag::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RecordTag::RecordId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(RecordTag::TagId).big_integer().not_null())
                    .col(
                        ColumnDef::new(RecordTag::Manual)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(RecordTag::CreateTime)
                            .date()
                            .not_null()
                            .default(Expr::current_date()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_tag_record")
                            .from(RecordTag::Table, RecordTag::RecordId)
                            .to(Record::Table, Record::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_record_tag_tag")
                            .from(RecordTag::Table, RecordTag::TagId)
                            .to(Tag::Table, Tag::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for record_id and tag_id combination
        manager
            .create_index(
                Index::create()
                    .name("idx_record_tag_unique")
                    .table(RecordTag::Table)
                    .col(RecordTag::RecordId)
                    .col(RecordTag::TagId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create index for tag name search
        manager
            .create_index(
                Index::create()
                    .name("idx_tag_name")
                    .table(Tag::Table)
                    .col(Tag::Name)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecordTag::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tag::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TagCategory::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum TagCategory {
    Table,
    Id,
    Name,
    Description,
    CreateTime,
}

#[derive(DeriveIden)]
enum Tag {
    Table,
    Id,
    CategoryId,
    Name,
    Description,
    CreateTime,
}

#[derive(DeriveIden)]
enum RecordTag {
    Table,
    Id,
    RecordId,
    TagId,
    Manual,
    CreateTime,
}

// Reference to other tables
#[derive(DeriveIden)]
enum Record {
    Table,
    Id,
}
