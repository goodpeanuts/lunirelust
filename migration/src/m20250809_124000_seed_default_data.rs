use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Insert default data for basic lookup tables

        // Default Director
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Director::Table)
                    .columns([Director::Id, Director::Name, Director::Link])
                    .values_panic([1.into(), "Unknown Director".into(), "".into()])
                    .to_owned(),
            )
            .await?;

        // Default Studio
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Studio::Table)
                    .columns([Studio::Id, Studio::Name, Studio::Link])
                    .values_panic([1.into(), "Unknown Studio".into(), "".into()])
                    .to_owned(),
            )
            .await?;

        // Default Label
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Label::Table)
                    .columns([Label::Id, Label::Name, Label::Link])
                    .values_panic([1.into(), "Unknown Label".into(), "".into()])
                    .to_owned(),
            )
            .await?;

        // Default Series
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Series::Table)
                    .columns([Series::Id, Series::Name, Series::Link])
                    .values_panic([1.into(), "Unknown Series".into(), "".into()])
                    .to_owned(),
            )
            .await?;

        // Insert some common tag categories
        let tag_categories = vec![
            (1, "Genre", "Content genre classification"),
            (2, "Actor", "Performers and actors"),
            (3, "Category", "General content categories"),
            (4, "Quality", "Video quality indicators"),
            (5, "Type", "Content type classification"),
        ];

        for (id, name, description) in tag_categories {
            manager
                .exec_stmt(
                    Query::insert()
                        .into_table(TagCategory::Table)
                        .columns([TagCategory::Id, TagCategory::Name, TagCategory::Description])
                        .values_panic([id.into(), name.into(), description.into()])
                        .to_owned(),
                )
                .await?;
        }

        // Insert some default tags
        let default_tags = vec![
            (1, 1, "Action", "Action content"),
            (2, 1, "Drama", "Dramatic content"),
            (3, 1, "Comedy", "Comedy content"),
            (4, 4, "HD", "High definition quality"),
            (5, 4, "4K", "Ultra high definition quality"),
            (6, 5, "Movie", "Full length movie"),
            (7, 5, "Series", "Series content"),
        ];

        for (id, category_id, name, description) in default_tags {
            manager
                .exec_stmt(
                    Query::insert()
                        .into_table(Tag::Table)
                        .columns([Tag::Id, Tag::CategoryId, Tag::Name, Tag::Description])
                        .values_panic([
                            id.into(),
                            category_id.into(),
                            name.into(),
                            description.into(),
                        ])
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove seed data in reverse order of dependencies

        // Remove tags
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Tag::Table)
                    .and_where(Expr::col(Tag::Id).lte(7))
                    .to_owned(),
            )
            .await?;

        // Remove tag categories
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(TagCategory::Table)
                    .and_where(Expr::col(TagCategory::Id).lte(5))
                    .to_owned(),
            )
            .await?;

        // Remove default lookup data
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Director::Table)
                    .and_where(Expr::col(Director::Id).eq(1))
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Studio::Table)
                    .and_where(Expr::col(Studio::Id).eq(1))
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Label::Table)
                    .and_where(Expr::col(Label::Id).eq(1))
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Series::Table)
                    .and_where(Expr::col(Series::Id).eq(1))
                    .to_owned(),
            )
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
}

#[derive(DeriveIden)]
enum Studio {
    Table,
    Id,
    Name,
    Link,
}

#[derive(DeriveIden)]
enum Label {
    Table,
    Id,
    Name,
    Link,
}

#[derive(DeriveIden)]
enum Series {
    Table,
    Id,
    Name,
    Link,
}

#[derive(DeriveIden)]
enum TagCategory {
    Table,
    Id,
    Name,
    Description,
}

#[derive(DeriveIden)]
enum Tag {
    Table,
    Id,
    CategoryId,
    Name,
    Description,
}
