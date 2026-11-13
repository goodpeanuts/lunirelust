//! Persisted invalidation for periods when the primary database is the only search backend.

use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, DbErr, Statement};

pub(super) struct IndexState {
    pub generation: i64,
    pub needs_rebuild: bool,
}

/// This single startup write replaces all per-mutation indexing work in `PostgreSQL` mode.
/// The caller must await it before accepting catalog writes.
pub(crate) async fn mark_postgres_startup(db: &DatabaseConnection) -> Result<(), DbErr> {
    let result = db
        .execute(Statement::from_string(
            DbBackend::Postgres,
            "UPDATE search_index_state SET generation = generation + 1 WHERE id = TRUE",
        ))
        .await?;
    if result.rows_affected() != 1 {
        return Err(DbErr::Custom(
            "search index state is missing; apply migrations before startup".into(),
        ));
    }
    Ok(())
}

pub(super) async fn read(db: &DatabaseConnection) -> Result<IndexState, DbErr> {
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT generation, indexed_generation FROM search_index_state WHERE id = TRUE",
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("search index state is missing".into()))?;
    let generation: i64 = row.try_get("", "generation")?;
    let indexed_generation: i64 = row.try_get("", "indexed_generation")?;
    Ok(IndexState {
        generation,
        needs_rebuild: generation != indexed_generation,
    })
}

/// Retire only events visible in this statement's snapshot before rebuilding from current data.
/// A failed rebuild leaves the generation dirty, so every retry repeats the full refresh.
/// Later commits remain pending even if their sequence IDs were allocated earlier.
pub(super) async fn retire_visible_backlog(db: &DatabaseConnection) -> Result<(), DbErr> {
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        "UPDATE search_sync_events SET processed_at = NOW(), claimed_by = NULL, claimed_at = NULL
         WHERE processed_at IS NULL",
    ))
    .await?;
    Ok(())
}

/// Do not acknowledge a newer `PostgreSQL` startup that arrived during the rebuild.
pub(super) async fn acknowledge(db: &DatabaseConnection, generation: i64) -> Result<bool, DbErr> {
    let result = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE search_index_state SET indexed_generation = $1 WHERE id = TRUE AND generation = $1",
        [generation.into()],
    )).await?;
    Ok(result.rows_affected() == 1)
}
