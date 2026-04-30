use crate::domains::crawl::domain::model::{
    CodeResultStatus, CrawlCodeResult, CrawlPageResult, CrawlTask, CrawlTaskDetail,
    PageResultStatus, TaskStatus, TaskType,
};
use crate::domains::crawl::domain::repository::CrawlTaskRepository;
use crate::entities::{crawl_code_result, crawl_page_result, crawl_task};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DbErr, EntityTrait as _,
    PaginatorTrait as _, QueryFilter as _, QueryOrder as _, QuerySelect as _, Set,
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
