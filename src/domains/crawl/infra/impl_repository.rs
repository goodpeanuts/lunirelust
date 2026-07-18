use crate::domains::crawl::domain::model::{
    CodeResultStatus, CrawlCodeResult, CrawlPageResult, CrawlTask, CrawlTaskDetail, CrawlTaskInput,
    EntityAutoCrawlScope, EntityAutoCrawlTaskInput, EntityAutoCrawlType, PageResultStatus,
    TaskStatus, TaskType,
};
use crate::domains::crawl::domain::repository::{
    ClaimedEntity, CrawlTaskRepository, EntityProgressRepository, EntityProgressRow,
    EntityProgressSummaryData,
};
use crate::entities::{crawl_code_result, crawl_page_result, crawl_task};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, ConnectionTrait as _, DatabaseBackend,
    DatabaseConnection, DbErr, EntityTrait as _, PaginatorTrait as _, QueryFilter as _,
    QueryOrder as _, QuerySelect as _, Set, Statement, TransactionTrait as _,
};

pub struct CrawlRepo;

// -- Entity-to-domain conversion helpers --

fn task_entity_to_domain(m: crawl_task::Model) -> Result<CrawlTask, DbErr> {
    Ok(CrawlTask {
        id: m.id,
        task_type: TaskType::from_str(&m.task_type)
            .ok_or_else(|| DbErr::Custom(format!("invalid task_type value: {}", m.task_type)))?,
        status: TaskStatus::from_str(&m.status)
            .ok_or_else(|| DbErr::Custom(format!("invalid status value: {}", m.status)))?,
        user_id: m.user_id,
        mark_liked: m.mark_liked,
        mark_viewed: m.mark_viewed,
        input_payload: m.input_payload,
        max_pages: m.max_pages,
        total_codes: m.total_codes,
        success_count: m.success_count,
        fail_count: m.fail_count,
        skip_count: m.skip_count,
        error_message: m.error_message,
        created_at: m.created_at,
        started_at: m.started_at,
        completed_at: m.completed_at,
    })
}

fn code_result_entity_to_domain(m: crawl_code_result::Model) -> Result<CrawlCodeResult, DbErr> {
    Ok(CrawlCodeResult {
        id: m.id,
        task_id: m.task_id,
        code: m.code,
        status: CodeResultStatus::from_str(&m.status)
            .ok_or_else(|| DbErr::Custom(format!("invalid code_result status: {}", m.status)))?,
        record_id: m.record_id,
        images_downloaded: m.images_downloaded,
        error_message: m.error_message,
        created_at: m.created_at,
    })
}

fn page_result_entity_to_domain(m: crawl_page_result::Model) -> Result<CrawlPageResult, DbErr> {
    Ok(CrawlPageResult {
        id: m.id,
        task_id: m.task_id,
        page_number: m.page_number,
        status: PageResultStatus::from_str(&m.status)
            .ok_or_else(|| DbErr::Custom(format!("invalid page_result status: {}", m.status)))?,
        records_found: m.records_found,
        records_crawled: m.records_crawled,
        error_message: m.error_message,
        created_at: m.created_at,
    })
}

#[async_trait]
impl CrawlTaskRepository for CrawlRepo {
    async fn create_task(
        &self,
        db: &DatabaseConnection,
        task_type: &TaskType,
        status: &TaskStatus,
        user_id: &str,
        mark_liked: bool,
        mark_viewed: bool,
        input_payload: Option<&str>,
        max_pages: Option<i32>,
        total_codes: i32,
    ) -> Result<CrawlTask, DbErr> {
        let now = Utc::now();
        let active = crawl_task::ActiveModel {
            task_type: Set(task_type.as_str().to_owned()),
            status: Set(status.as_str().to_owned()),
            user_id: Set(user_id.to_owned()),
            mark_liked: Set(mark_liked),
            mark_viewed: Set(mark_viewed),
            input_payload: Set(input_payload.map(|s| s.to_owned())),
            max_pages: Set(max_pages),
            total_codes: Set(total_codes),
            success_count: Set(0),
            fail_count: Set(0),
            skip_count: Set(0),
            error_message: Set(None),
            created_at: Set(now),
            started_at: Set(None),
            completed_at: Set(None),
            ..Default::default()
        };

        let inserted = active.insert(db).await?;
        task_entity_to_domain(inserted)
    }

    async fn update_task_status(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        status: &TaskStatus,
        error_message: Option<&str>,
    ) -> Result<(), DbErr> {
        let task: Option<crawl_task::Model> =
            crawl_task::Entity::find_by_id(task_id).one(db).await?;

        let task = task.ok_or_else(|| DbErr::Custom(format!("task {task_id} not found")))?;
        let mut active: crawl_task::ActiveModel = task.into();

        active.status = Set(status.as_str().to_owned());
        active.error_message = Set(error_message.map(|s| s.to_owned()));

        active.update(db).await?;
        Ok(())
    }

    async fn update_task_started(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<(), DbErr> {
        let task: Option<crawl_task::Model> =
            crawl_task::Entity::find_by_id(task_id).one(db).await?;

        let task = task.ok_or_else(|| DbErr::Custom(format!("task {task_id} not found")))?;
        let mut active: crawl_task::ActiveModel = task.into();

        active.status = Set("running".to_owned());
        active.started_at = Set(Some(Utc::now()));

        active.update(db).await?;
        Ok(())
    }

    async fn update_task_counts(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        success_count: i32,
        fail_count: i32,
        skip_count: i32,
        total_codes: i32,
    ) -> Result<(), DbErr> {
        let task: Option<crawl_task::Model> =
            crawl_task::Entity::find_by_id(task_id).one(db).await?;

        let task = task.ok_or_else(|| DbErr::Custom(format!("task {task_id} not found")))?;
        let mut active: crawl_task::ActiveModel = task.into();

        active.success_count = Set(success_count);
        active.fail_count = Set(fail_count);
        active.skip_count = Set(skip_count);
        active.total_codes = Set(total_codes);

        active.update(db).await?;
        Ok(())
    }

    async fn complete_task(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        status: &TaskStatus,
        success_count: i32,
        fail_count: i32,
        skip_count: i32,
        total_codes: i32,
        error_message: Option<&str>,
    ) -> Result<(), DbErr> {
        let task: Option<crawl_task::Model> =
            crawl_task::Entity::find_by_id(task_id).one(db).await?;

        let task = task.ok_or_else(|| DbErr::Custom(format!("task {task_id} not found")))?;
        let mut active: crawl_task::ActiveModel = task.into();

        active.status = Set(status.as_str().to_owned());
        active.success_count = Set(success_count);
        active.fail_count = Set(fail_count);
        active.skip_count = Set(skip_count);
        active.total_codes = Set(total_codes);
        active.error_message = Set(error_message.map(|s| s.to_owned()));
        active.completed_at = Set(Some(Utc::now()));

        active.update(db).await?;
        Ok(())
    }

    async fn get_task_by_id(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Option<CrawlTask>, DbErr> {
        crawl_task::Entity::find_by_id(task_id)
            .one(db)
            .await?
            .map(task_entity_to_domain)
            .transpose()
    }

    async fn list_tasks(
        &self,
        db: &DatabaseConnection,
        user_id: &str,
        status_filter: Option<&TaskStatus>,
        task_type_filter: Option<&TaskType>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<CrawlTask>, u64), DbErr> {
        let mut query = crawl_task::Entity::find().filter(crawl_task::Column::UserId.eq(user_id));

        if let Some(status) = status_filter {
            query = query.filter(crawl_task::Column::Status.eq(status.as_str()));
        }
        if let Some(task_type) = task_type_filter {
            query = query.filter(crawl_task::Column::TaskType.eq(task_type.as_str()));
        }

        let total = query.clone().count(db).await?;

        let tasks: Vec<crawl_task::Model> = query
            .order_by_desc(crawl_task::Column::CreatedAt)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(db)
            .await?;

        let tasks = tasks
            .into_iter()
            .map(task_entity_to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((tasks, total))
    }

    async fn create_code_result(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        code: &str,
        status: &str,
        record_id: Option<&str>,
        images_downloaded: i32,
        error_message: Option<&str>,
    ) -> Result<CrawlCodeResult, DbErr> {
        let active = crawl_code_result::ActiveModel {
            task_id: Set(task_id),
            code: Set(code.to_owned()),
            status: Set(status.to_owned()),
            record_id: Set(record_id.map(|s| s.to_owned())),
            images_downloaded: Set(images_downloaded),
            error_message: Set(error_message.map(|s| s.to_owned())),
            ..Default::default()
        };

        let inserted = active.insert(db).await?;
        code_result_entity_to_domain(inserted)
    }

    async fn list_code_results(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Vec<CrawlCodeResult>, DbErr> {
        crawl_code_result::Entity::find()
            .filter(crawl_code_result::Column::TaskId.eq(task_id))
            .all(db)
            .await?
            .into_iter()
            .map(code_result_entity_to_domain)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn create_page_result(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        page_number: i32,
        status: &PageResultStatus,
        records_found: i32,
        error_message: Option<&str>,
    ) -> Result<CrawlPageResult, DbErr> {
        let active = crawl_page_result::ActiveModel {
            task_id: Set(task_id),
            page_number: Set(page_number),
            status: Set(status.as_str().to_owned()),
            records_found: Set(records_found),
            records_crawled: Set(0),
            error_message: Set(error_message.map(|s| s.to_owned())),
            ..Default::default()
        };

        let inserted = active.insert(db).await?;
        page_result_entity_to_domain(inserted)
    }

    async fn update_page_result(
        &self,
        db: &DatabaseConnection,
        id: i64,
        status: &PageResultStatus,
        records_crawled: i32,
        error_message: Option<&str>,
    ) -> Result<(), DbErr> {
        let row: Option<crawl_page_result::Model> =
            crawl_page_result::Entity::find_by_id(id).one(db).await?;

        let row = row.ok_or_else(|| DbErr::Custom(format!("page_result {id} not found")))?;
        let mut active: crawl_page_result::ActiveModel = row.into();

        active.status = Set(status.as_str().to_owned());
        active.records_crawled = Set(records_crawled);
        active.error_message = Set(error_message.map(|s| s.to_owned()));

        active.update(db).await?;
        Ok(())
    }

    async fn list_page_results(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Vec<CrawlPageResult>, DbErr> {
        crawl_page_result::Entity::find()
            .filter(crawl_page_result::Column::TaskId.eq(task_id))
            .all(db)
            .await?
            .into_iter()
            .map(page_result_entity_to_domain)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn find_tasks_by_status(
        &self,
        db: &DatabaseConnection,
        statuses: &[TaskStatus],
    ) -> Result<Vec<CrawlTask>, DbErr> {
        let status_strs: Vec<&str> = statuses.iter().map(|s| s.as_str()).collect();

        crawl_task::Entity::find()
            .filter(crawl_task::Column::Status.is_in(status_strs))
            .order_by_asc(crawl_task::Column::CreatedAt)
            .all(db)
            .await?
            .into_iter()
            .map(task_entity_to_domain)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn fail_processing_page_results(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
        error_message: &str,
    ) -> Result<(), DbErr> {
        let rows = crawl_page_result::Entity::find()
            .filter(crawl_page_result::Column::TaskId.eq(task_id))
            .filter(crawl_page_result::Column::Status.eq("processing"))
            .all(db)
            .await?;

        for row in rows {
            let mut active: crawl_page_result::ActiveModel = row.into();
            active.status = Set("failed".to_owned());
            active.error_message = Set(Some(error_message.to_owned()));
            active.update(db).await?;
        }

        Ok(())
    }

    async fn get_task_detail(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<Option<CrawlTaskDetail>, DbErr> {
        let task = crawl_task::Entity::find_by_id(task_id)
            .one(db)
            .await?
            .map(task_entity_to_domain)
            .transpose()?;

        let Some(task) = task else {
            return Ok(None);
        };

        let code_results = crawl_code_result::Entity::find()
            .filter(crawl_code_result::Column::TaskId.eq(task_id))
            .all(db)
            .await?
            .into_iter()
            .map(code_result_entity_to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        let page_results = crawl_page_result::Entity::find()
            .filter(crawl_page_result::Column::TaskId.eq(task_id))
            .all(db)
            .await?
            .into_iter()
            .map(page_result_entity_to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(CrawlTaskDetail {
            task,
            code_results,
            page_results,
        }))
    }

    async fn count_code_results_by_status(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<(i32, i32, i32), DbErr> {
        let results = crawl_code_result::Entity::find()
            .filter(crawl_code_result::Column::TaskId.eq(task_id))
            .all(db)
            .await?;

        let mut success = 0i32;
        let mut failed = 0i32;
        let mut skipped = 0i32;

        for r in &results {
            match r.status.as_str() {
                "success" | "partial" => success += 1,
                "failed" => failed += 1,
                "skipped" => skipped += 1,
                _ => {}
            }
        }

        Ok((success, failed, skipped))
    }
}

/// Acquire a transaction-scoped advisory lock keyed by entity type, serializing
/// same-type entity-auto-crawl claims so two concurrent requests cannot both
/// select and bump the same entity. Different entity types lock independently.
async fn acquire_type_advisory_lock(
    txn: &sea_orm::DatabaseTransaction,
    entity_type: EntityAutoCrawlType,
) -> Result<(), DbErr> {
    let key = format!("entity_auto_crawl:{}", entity_type.as_str());
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtext($1))",
        [key.into()],
    ))
    .await?;
    Ok(())
}

/// Build the optional `crawl_task.status` filter clause for `list_progress`.
/// `status` is validated upstream to one of the four known values.
fn list_status_filter_clause(status: Option<&str>) -> &'static str {
    match status {
        Some("never") => " AND (t.id IS NULL OR t.status = 'cancelled')",
        Some("in_progress") => " AND t.status IN ('queued', 'running')",
        Some("completed") => " AND t.status = 'completed'",
        Some("failed") => " AND t.status = 'failed'",
        _ => "",
    }
}

/// Serialize an `EntityAutoCrawlTaskInput` into the `crawl_task.input_payload`
/// JSON string.
fn serialize_entity_auto_payload(input: &EntityAutoCrawlTaskInput) -> Result<String, DbErr> {
    // Wrap in the tagged `CrawlTaskInput` enum so the persisted JSON carries the
    // `"type":"entity_auto_crawl"` discriminator that the read-back deserializer
    // (`CrawlTaskInput`, `#[serde(tag = "type")]`) requires. Serializing the bare
    // struct omits the tag and makes the task fail with "Invalid persisted payload".
    serde_json::to_string(&CrawlTaskInput::EntityAutoCrawl(input.clone()))
        .map_err(|e| DbErr::Custom(format!("failed to serialize entity-auto input: {e}")))
}

#[async_trait]
impl EntityProgressRepository for CrawlRepo {
    async fn current_round(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
    ) -> Result<i64, DbErr> {
        // Model D: MIN over ALL link<>'' entities via LEFT JOIN, treating a
        // row-less entity as round 0 (COALESCE). Row-less entities participate in
        // the MIN so a fresh type is current_round = 0.
        let sql = format!(
            "SELECT COALESCE(MIN(COALESCE(p.last_crawled_round, 0)), 0)::bigint AS cr \
             FROM {} e \
             LEFT JOIN crawl_entity_progress p ON p.entity_type = $1 AND p.entity_id = e.id \
             WHERE e.link <> ''",
            entity_type.table_name()
        );
        let row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql.as_str(),
                [entity_type.as_str().into()],
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("current_round query returned no row".to_owned()))?;
        let cr: i64 = row.try_get("", "cr")?;
        Ok(cr)
    }

    async fn claim_uncrawled(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        count: u32,
        current_round: i64,
        user_id: &str,
        base_url: &str,
    ) -> Result<Vec<ClaimedEntity>, DbErr> {
        let type_str = entity_type.as_str().to_owned();
        let select_sql = format!(
            "SELECT e.id, e.name, e.link FROM {} e \
             LEFT JOIN crawl_entity_progress p ON p.entity_type = $1 AND p.entity_id = e.id \
             LEFT JOIN crawl_task t ON t.id = p.last_task_id \
             WHERE e.link <> '' \
             AND COALESCE(p.last_crawled_round, $2::int) = $2::int \
             AND (t.id IS NULL OR t.status NOT IN ('queued', 'running')) \
             ORDER BY e.id ASC LIMIT $3::bigint",
            entity_type.table_name()
        );

        let txn = db.begin().await?;
        acquire_type_advisory_lock(&txn, entity_type).await?;

        let rows = txn
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                select_sql.as_str(),
                [
                    type_str.clone().into(),
                    current_round.into(),
                    (count as i64).into(),
                ],
            ))
            .await?;

        let mut claimed = Vec::new();
        for row in rows {
            let entity_id: i64 = row.try_get("", "id")?;
            let entity_name: String = row.try_get("", "name")?;
            let link: String = row.try_get("", "link")?;

            let input = EntityAutoCrawlTaskInput {
                entity_type,
                entity_id,
                entity_name: entity_name.clone(),
                link: link.trim_end_matches('/').to_owned(),
                base_url: base_url.to_owned(),
                crawl_round: current_round,
                scope: EntityAutoCrawlScope::Uncrawled,
            };
            let payload = serialize_entity_auto_payload(&input)?;

            let task_row = txn
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "INSERT INTO crawl_task \
                     (task_type, status, user_id, mark_liked, mark_viewed, input_payload, \
                     max_pages, total_codes, success_count, fail_count, skip_count, \
                     error_message, created_at, started_at, completed_at) \
                     VALUES ('entity_auto_crawl', 'queued', $1, false, false, $2, NULL, 0, 0, 0, 0, NULL, now(), NULL, NULL) \
                     RETURNING id",
                    [user_id.into(), payload.into()],
                ))
                .await?;
            let task_id: i64 = task_row
                .ok_or_else(|| DbErr::Custom("crawl_task insert returned no row".to_owned()))?
                .try_get("", "id")?;

            // Model D: claim never advances the round. Only set last_task_id.
            // A new row is inserted at last_crawled_round = 0; an existing row's
            // round is left unchanged (it advances only on successful complete).
            txn.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "INSERT INTO crawl_entity_progress \
                 (entity_type, entity_id, entity_name, last_crawled_round, last_task_id, created_at, updated_at) \
                 VALUES ($1, $2, $3, 0, $4, now(), now()) \
                 ON CONFLICT (entity_type, entity_id) DO UPDATE SET \
                 last_task_id = $4, entity_name = $3, updated_at = now()",
                [
                    type_str.clone().into(),
                    entity_id.into(),
                    entity_name.clone().into(),
                    task_id.into(),
                ],
            ))
            .await?;

            claimed.push(ClaimedEntity {
                entity_id,
                entity_name,
                task_id,
            });
        }

        txn.commit().await?;
        Ok(claimed)
    }

    async fn claim_failed(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        count: u32,
        current_round: i64,
        user_id: &str,
        base_url: &str,
    ) -> Result<Vec<ClaimedEntity>, DbErr> {
        let type_str = entity_type.as_str().to_owned();
        let select_sql = format!(
            "SELECT e.id, e.name, e.link FROM {} e \
             JOIN crawl_entity_progress p ON p.entity_type = $1 AND p.entity_id = e.id \
             JOIN crawl_task t ON t.id = p.last_task_id \
             WHERE e.link <> '' AND t.status = 'failed' \
             ORDER BY e.id ASC LIMIT $2::bigint",
            entity_type.table_name()
        );

        let txn = db.begin().await?;
        acquire_type_advisory_lock(&txn, entity_type).await?;

        let rows = txn
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                select_sql.as_str(),
                [type_str.clone().into(), (count as i64).into()],
            ))
            .await?;

        let mut claimed = Vec::new();
        for row in rows {
            let entity_id: i64 = row.try_get("", "id")?;
            let entity_name: String = row.try_get("", "name")?;
            let link: String = row.try_get("", "link")?;

            let input = EntityAutoCrawlTaskInput {
                entity_type,
                entity_id,
                entity_name: entity_name.clone(),
                link: link.trim_end_matches('/').to_owned(),
                base_url: base_url.to_owned(),
                crawl_round: current_round,
                scope: EntityAutoCrawlScope::Failed,
            };
            let payload = serialize_entity_auto_payload(&input)?;

            let task_row = txn
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "INSERT INTO crawl_task \
                     (task_type, status, user_id, mark_liked, mark_viewed, input_payload, \
                     max_pages, total_codes, success_count, fail_count, skip_count, \
                     error_message, created_at, started_at, completed_at) \
                     VALUES ('entity_auto_crawl', 'queued', $1, false, false, $2, NULL, 0, 0, 0, 0, NULL, now(), NULL, NULL) \
                     RETURNING id",
                    [user_id.into(), payload.into()],
                ))
                .await?;
            let task_id: i64 = task_row
                .ok_or_else(|| DbErr::Custom("crawl_task insert returned no row".to_owned()))?
                .try_get("", "id")?;

            // Failed claim: round-neutral. Only reassign last_task_id; the
            // INSERT arm (defensive, normally unreachable for a failed entity)
            // writes current_round so it never lowers the MIN.
            txn.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "INSERT INTO crawl_entity_progress \
                 (entity_type, entity_id, entity_name, last_crawled_round, last_task_id, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, now(), now()) \
                 ON CONFLICT (entity_type, entity_id) DO UPDATE SET \
                 last_task_id = $5, entity_name = $3, updated_at = now()",
                [
                    type_str.clone().into(),
                    entity_id.into(),
                    entity_name.clone().into(),
                    current_round.into(),
                    task_id.into(),
                ],
            ))
            .await?;

            claimed.push(ClaimedEntity {
                entity_id,
                entity_name,
                task_id,
            });
        }

        txn.commit().await?;
        Ok(claimed)
    }

    async fn count_remaining(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        current_round: i64,
    ) -> Result<u64, DbErr> {
        let sql = format!(
            "SELECT count(*)::bigint AS cnt FROM {} e \
             LEFT JOIN crawl_entity_progress p ON p.entity_type = $1 AND p.entity_id = e.id \
             LEFT JOIN crawl_task t ON t.id = p.last_task_id \
             WHERE e.link <> '' \
             AND COALESCE(p.last_crawled_round, $2::int) = $2::int \
             AND (t.id IS NULL OR t.status NOT IN ('queued', 'running'))",
            entity_type.table_name()
        );
        let row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql.as_str(),
                [entity_type.as_str().into(), current_round.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("count_remaining query returned no row".to_owned()))?;
        let cnt: i64 = row.try_get("", "cnt")?;
        Ok(cnt.max(0) as u64)
    }

    async fn advance_round_on_complete(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        entity_id: i64,
        crawl_round: i64,
    ) -> Result<(), DbErr> {
        // Model D: the ONLY writer of last_crawled_round. Monotonic and
        // idempotent via GREATEST, so out-of-order or duplicate completions
        // never lower the round.
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE crawl_entity_progress \
             SET last_crawled_round = GREATEST(last_crawled_round, $3::int), \
                 last_crawled_at = now(), updated_at = now() \
             WHERE entity_type = $1 AND entity_id = $2",
            [
                entity_type.as_str().into(),
                entity_id.into(),
                (crawl_round + 1).into(),
            ],
        ))
        .await?;
        Ok(())
    }

    async fn cancel_queued_entity_auto_task(
        &self,
        db: &DatabaseConnection,
        task_id: i64,
    ) -> Result<(), DbErr> {
        // Model D: cancellation only marks the task cancelled; it never touches
        // last_crawled_round. The entity stays at its round (= MIN) and is
        // immediately re-selectable via the uncrawled scope, so no clamp and no
        // multi-statement atomicity concern.
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE crawl_task SET status = 'cancelled', completed_at = now() WHERE id = $1",
            [task_id.into()],
        ))
        .await?;
        Ok(())
    }

    async fn touch_on_finalize(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        entity_id: i64,
    ) -> Result<(), DbErr> {
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE crawl_entity_progress \
             SET last_crawled_at = now(), updated_at = now() \
             WHERE entity_type = $1 AND entity_id = $2",
            [entity_type.as_str().into(), entity_id.into()],
        ))
        .await?;
        Ok(())
    }

    async fn progress_summary(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
    ) -> Result<EntityProgressSummaryData, DbErr> {
        let tbl = entity_type.table_name();
        // Model D: current_round = MIN over ALL link<>'' entities via LEFT JOIN,
        // treating a row-less entity as round 0 (so a fresh type is round 0). The
        // main query aggregates over the same link<>'' base so remaining never
        // exceeds total.
        let sql = format!(
            "WITH cr AS ( \
               SELECT COALESCE(MIN(COALESCE(p.last_crawled_round, 0)), 0)::int AS current_round \
               FROM {tbl} e \
               LEFT JOIN crawl_entity_progress p ON p.entity_type = $1 AND p.entity_id = e.id \
               WHERE e.link <> '' \
             ) \
             SELECT cr.current_round, \
               count(*)::bigint AS total, \
               count(*) FILTER ( \
                 WHERE COALESCE(p.last_crawled_round, cr.current_round) = cr.current_round \
                   AND (t.id IS NULL OR t.status NOT IN ('queued', 'running')) \
               )::bigint AS remaining, \
               count(*) FILTER (WHERE t.status = 'failed')::bigint AS failed \
             FROM {tbl} e CROSS JOIN cr \
             LEFT JOIN crawl_entity_progress p ON p.entity_type = $1 AND p.entity_id = e.id \
             LEFT JOIN crawl_task t ON t.id = p.last_task_id \
             WHERE e.link <> '' \
             GROUP BY cr.current_round",
        );
        let row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql.as_str(),
                [entity_type.as_str().into()],
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("progress_summary query returned no row".to_owned()))?;
        let current_round: i32 = row.try_get("", "current_round")?;
        let total: i64 = row.try_get("", "total")?;
        let remaining: i64 = row.try_get("", "remaining")?;
        let failed: i64 = row.try_get("", "failed")?;
        Ok(EntityProgressSummaryData {
            current_round: current_round as i64,
            total: total.max(0) as u64,
            remaining: remaining.max(0) as u64,
            failed: failed.max(0) as u64,
        })
    }

    async fn list_progress(
        &self,
        db: &DatabaseConnection,
        entity_type: EntityAutoCrawlType,
        status: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<EntityProgressRow>, u64), DbErr> {
        let tbl = entity_type.table_name();
        let filter = list_status_filter_clause(status);
        let limit = page_size as i64;
        let offset = ((page.saturating_sub(1)) * page_size) as i64;

        let base = format!(
            "FROM {tbl} e \
             LEFT JOIN crawl_entity_progress p ON p.entity_type = $1 AND p.entity_id = e.id \
             LEFT JOIN crawl_task t ON t.id = p.last_task_id \
             WHERE e.link <> ''{filter}",
        );

        let list_sql = format!(
            "SELECT e.id, e.name, \
               CASE WHEN t.id IS NULL OR t.status = 'cancelled' THEN 'never' \
                    WHEN t.status IN ('queued', 'running') THEN 'in_progress' \
                    WHEN t.status = 'completed' THEN 'completed' \
                    WHEN t.status = 'failed' THEN 'failed' \
                    ELSE 'never' END AS derived_status, \
               p.last_crawled_round, p.last_crawled_at, p.last_task_id \
             {base} ORDER BY e.id ASC LIMIT $2::bigint OFFSET $3::bigint",
        );
        let rows = db
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                list_sql.as_str(),
                [entity_type.as_str().into(), limit.into(), offset.into()],
            ))
            .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let entity_id: i64 = row.try_get("", "id")?;
            let entity_name: String = row.try_get("", "name")?;
            let status_str: String = row.try_get("", "derived_status")?;
            let last_crawled_round: Option<i32> = row.try_get("", "last_crawled_round")?;
            let last_crawled_at: Option<DateTime<Utc>> = row.try_get("", "last_crawled_at")?;
            let last_task_id: Option<i64> = row.try_get("", "last_task_id")?;
            items.push(EntityProgressRow {
                entity_id,
                entity_name,
                status: status_str,
                last_crawled_round,
                last_crawled_at,
                last_task_id,
            });
        }

        let count_sql = format!("SELECT count(*)::bigint AS cnt {base}");
        let count_row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                count_sql.as_str(),
                [entity_type.as_str().into()],
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("list_progress count returned no row".to_owned()))?;
        let total: i64 = count_row.try_get("", "cnt")?;

        Ok((items, total.max(0) as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::{serialize_entity_auto_payload, EntityAutoCrawlScope, EntityAutoCrawlType};
    use crate::domains::crawl::domain::model::{CrawlTaskInput, EntityAutoCrawlTaskInput};

    /// The persisted payload must round-trip through the tagged `CrawlTaskInput`
    /// deserializer. Regression guard for the "Invalid persisted payload" bug,
    /// where the payload was serialized as a bare struct without the
    /// `"type":"entity_auto_crawl"` discriminator and failed to read back.
    #[test]
    fn entity_auto_payload_roundtrips_through_tagged_enum() {
        let input = EntityAutoCrawlTaskInput {
            entity_type: EntityAutoCrawlType::Idol,
            entity_id: 42,
            entity_name: "test-idol".to_owned(),
            link: "https://www.example.com/star/mock".to_owned(),
            base_url: "https://www.example.com".to_owned(),
            crawl_round: 1,
            scope: EntityAutoCrawlScope::Uncrawled,
        };

        let json = serialize_entity_auto_payload(&input).expect("serialize");
        // The tag must be present, otherwise the read-back deserializer rejects it.
        assert!(
            json.contains("\"type\":\"entity_auto_crawl\""),
            "payload missing type tag: {json}"
        );

        // Must deserialize back through the tagged enum the runner uses.
        let parsed: CrawlTaskInput = serde_json::from_str(&json).expect("deserialize");
        match parsed {
            CrawlTaskInput::EntityAutoCrawl(out) => {
                assert_eq!(out.entity_id, 42);
                assert_eq!(out.entity_type, EntityAutoCrawlType::Idol);
                assert_eq!(out.scope, EntityAutoCrawlScope::Uncrawled);
            }
            other => panic!("expected EntityAutoCrawl variant, got {other:?}"),
        }
    }
}
