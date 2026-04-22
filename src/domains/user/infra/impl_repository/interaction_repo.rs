use crate::domains::user::{
    domain::model::user_interaction::InteractionStatus,
    domain::repository::interaction_repo::InteractionRepository,
};
use crate::entities::user_record_interaction;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::{Alias, Expr, OnConflict},
    ActiveValue::Set,
    ColumnTrait as _, DatabaseConnection, DbErr, EntityTrait as _, PaginatorTrait as _,
    QueryFilter as _, QueryOrder as _, QuerySelect as _,
};

pub struct InteractionRepo;

#[async_trait]
impl InteractionRepository for InteractionRepo {
    /// Atomically toggle the liked status for a user/record pair.
    ///
    /// Uses `ON CONFLICT (user_id, record_id) DO UPDATE SET
    ///   liked = NOT <table>.liked,
    ///   liked_at = CASE WHEN NOT <table>.liked THEN <now> ELSE NULL END`
    /// so the toggle is a single statement and safe under concurrency.
    /// The existing `viewed`/`viewed_at` columns are preserved on conflict.
    async fn toggle_like(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        record_id: &str,
    ) -> Result<bool, DbErr> {
        let now = Utc::now();
        let table_name = Alias::new("user_record_interaction");
        let liked_col = Expr::col((table_name.clone(), user_record_interaction::Column::Liked));

        let active = user_record_interaction::ActiveModel {
            user_id: Set(user_id.to_owned()),
            record_id: Set(record_id.to_owned()),
            liked: Set(true),
            liked_at: Set(Some(now)),
            viewed: Set(false),
            viewed_at: Set(None),
            ..Default::default()
        };

        let on_conflict = OnConflict::columns([
            user_record_interaction::Column::UserId,
            user_record_interaction::Column::RecordId,
        ])
        .value(
            user_record_interaction::Column::Liked,
            liked_col.clone().not(),
        )
        .value(
            user_record_interaction::Column::LikedAt,
            Expr::case(liked_col.not(), Expr::value(Some(now)))
                .finally(Expr::value(Option::<chrono::DateTime<Utc>>::None)),
        )
        .to_owned();

        user_record_interaction::Entity::insert(active)
            .on_conflict(on_conflict)
            .exec(db)
            .await?;

        // Read back the final state to return the authoritative liked value.
        let row = user_record_interaction::Entity::find()
            .filter(user_record_interaction::Column::UserId.eq(user_id))
            .filter(user_record_interaction::Column::RecordId.eq(record_id))
            .one(db)
            .await?
            .ok_or_else(|| {
                DbErr::Custom(format!(
                    "missing user_record_interaction row after upsert for user_id={user_id} record_id={record_id}"
                ))
            })?;

        Ok(row.liked)
    }

    /// Idempotently mark a user/record pair as viewed.
    ///
    /// Uses `ON CONFLICT (user_id, record_id) DO UPDATE SET viewed = TRUE`
    /// so concurrent first-view requests do not fail on the unique constraint.
    /// The existing `liked`/`liked_at` columns are preserved on conflict.
    async fn mark_viewed(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        record_id: &str,
    ) -> Result<(), DbErr> {
        let now = Utc::now();

        let active = user_record_interaction::ActiveModel {
            user_id: Set(user_id.to_owned()),
            record_id: Set(record_id.to_owned()),
            liked: Set(false),
            liked_at: Set(None),
            viewed: Set(true),
            viewed_at: Set(Some(now)),
            ..Default::default()
        };

        // On conflict: set viewed=TRUE, but only set viewed_at when not already viewed.
        // This keeps the first-view timestamp and makes the endpoint idempotent.
        let viewed_at_col = Expr::col((
            Alias::new("user_record_interaction"),
            user_record_interaction::Column::ViewedAt,
        ));
        let viewed_is_true = Expr::col((
            Alias::new("user_record_interaction"),
            user_record_interaction::Column::Viewed,
        ))
        .eq(true);
        let on_conflict = OnConflict::columns([
            user_record_interaction::Column::UserId,
            user_record_interaction::Column::RecordId,
        ])
        .value(user_record_interaction::Column::Viewed, Expr::value(true))
        .value(
            user_record_interaction::Column::ViewedAt,
            Expr::case(viewed_is_true, viewed_at_col).finally(Expr::value(Some(now))),
        )
        .to_owned();

        user_record_interaction::Entity::insert(active)
            .on_conflict(on_conflict)
            .exec(db)
            .await?;

        Ok(())
    }

    async fn batch_get_status(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        record_ids: &[String],
    ) -> Result<std::collections::HashMap<String, InteractionStatus>, DbErr> {
        if record_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let interactions = user_record_interaction::Entity::find()
            .filter(user_record_interaction::Column::UserId.eq(user_id))
            .filter(user_record_interaction::Column::RecordId.is_in(record_ids.to_vec()))
            .all(db)
            .await?;

        let mut status_map: std::collections::HashMap<String, InteractionStatus> = record_ids
            .iter()
            .map(|id| (id.clone(), InteractionStatus::default()))
            .collect();

        for interaction in interactions {
            status_map.insert(
                interaction.record_id.clone(),
                InteractionStatus {
                    liked: interaction.liked,
                    viewed: interaction.viewed,
                },
            );
        }

        Ok(status_map)
    }

    async fn find_viewed_record_ids_paginated(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(Vec<String>, u64), DbErr> {
        let query = user_record_interaction::Entity::find()
            .filter(user_record_interaction::Column::UserId.eq(user_id))
            .filter(user_record_interaction::Column::Viewed.eq(true))
            .order_by_desc(user_record_interaction::Column::ViewedAt);

        let total = query.clone().count(db).await?;

        let interactions: Vec<user_record_interaction::Model> =
            query.offset(offset).limit(limit).all(db).await?;

        let ids = interactions.into_iter().map(|i| i.record_id).collect();
        Ok((ids, total))
    }
}
