use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// --- Request DTOs ---

fn default_false() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct StartBatchRequest {
    #[validate(length(min = 1, message = "Codes list must not be empty"))]
    pub codes: Vec<String>,
    #[serde(default = "default_false")]
    pub mark_liked: bool,
    #[serde(default = "default_false")]
    pub mark_viewed: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct StartAutoRequest {
    #[validate(length(min = 1, message = "start_url must not be empty"))]
    pub start_url: String,
    #[validate(range(min = 1, message = "max_pages must be at least 1"))]
    pub max_pages: u32,
    #[serde(default = "default_false")]
    pub mark_liked: bool,
    #[serde(default = "default_false")]
    pub mark_viewed: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartUpdateRequest {
    #[serde(default = "default_false")]
    pub liked_only: bool,
    pub created_after: Option<String>,
}

impl Validate for StartUpdateRequest {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        if !self.liked_only && self.created_after.is_none() {
            errors.add(
                "filters",
                validator::ValidationError::new(
                    "Update mode requires at least one filter (liked_only or created_after)",
                ),
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// --- Response DTOs ---

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskResponse {
    pub task_id: i64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskListItem {
    pub id: i64,
    pub task_type: String,
    pub status: String,
    pub total_codes: i32,
    pub success_count: i32,
    pub fail_count: i32,
    pub skip_count: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CodeResultResponse {
    pub code: String,
    pub status: String,
    pub record_id: Option<String>,
    pub images_downloaded: i32,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PageResultResponse {
    pub page_number: i32,
    pub status: String,
    pub records_found: i32,
    pub records_crawled: i32,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskDetailResponse {
    #[serde(flatten)]
    pub task: TaskListItem,
    pub code_results: Vec<CodeResultResponse>,
    pub page_results: Vec<PageResultResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskListItem>,
    pub total: u64,
}

// --- SSE Event DTOs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum SseEvent {
    #[serde(rename = "task:started")]
    TaskStarted {
        task_id: i64,
        user_id: String,
        task_type: String,
    },
    #[serde(rename = "task:page:start")]
    PageStart {
        task_id: i64,
        user_id: String,
        page_number: i32,
    },
    #[serde(rename = "task:code:progress")]
    CodeProgress {
        task_id: i64,
        user_id: String,
        code: String,
        status: String,
        record_id: Option<String>,
        images_downloaded: i32,
    },
    #[serde(rename = "task:stats")]
    Stats {
        task_id: i64,
        user_id: String,
        success_count: i32,
        fail_count: i32,
        skip_count: i32,
        total: i32,
    },
    #[serde(rename = "task:page:complete")]
    PageComplete {
        task_id: i64,
        user_id: String,
        page_number: i32,
        records_found: i32,
        records_crawled: i32,
    },
    #[serde(rename = "task:completed")]
    TaskCompleted {
        task_id: i64,
        user_id: String,
        status: String,
        summary: TaskSummary,
    },
    #[serde(rename = "task:cancelled")]
    TaskCancelled {
        task_id: i64,
        user_id: String,
        summary: TaskSummary,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub total: i32,
    pub success: i32,
    pub failed: i32,
    pub skipped: i32,
    pub pages_crawled: i64,
}

impl SseEvent {
    pub fn task_id(&self) -> i64 {
        match self {
            Self::TaskStarted { task_id, .. }
            | Self::PageStart { task_id, .. }
            | Self::CodeProgress { task_id, .. }
            | Self::Stats { task_id, .. }
            | Self::PageComplete { task_id, .. }
            | Self::TaskCompleted { task_id, .. }
            | Self::TaskCancelled { task_id, .. } => *task_id,
        }
    }

    pub fn user_id(&self) -> &str {
        match self {
            Self::TaskStarted { user_id, .. }
            | Self::PageStart { user_id, .. }
            | Self::CodeProgress { user_id, .. }
            | Self::Stats { user_id, .. }
            | Self::PageComplete { user_id, .. }
            | Self::TaskCompleted { user_id, .. }
            | Self::TaskCancelled { user_id, .. } => user_id,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::TaskCompleted { .. } | Self::TaskCancelled { .. }
        )
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::TaskStarted { .. } => "task:started",
            Self::PageStart { .. } => "task:page:start",
            Self::CodeProgress { .. } => "task:code:progress",
            Self::Stats { .. } => "task:stats",
            Self::PageComplete { .. } => "task:page:complete",
            Self::TaskCompleted { .. } => "task:completed",
            Self::TaskCancelled { .. } => "task:cancelled",
        }
    }
}

// --- Query params ---

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub status: Option<String>,
    pub task_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batch_request_validates_empty_codes() {
        let req = StartBatchRequest {
            codes: vec![],
            mark_liked: false,
            mark_viewed: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn batch_request_accepts_valid_codes() {
        let req = StartBatchRequest {
            codes: vec!["ABC-123".to_owned()],
            mark_liked: true,
            mark_viewed: false,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn auto_request_validates_zero_max_pages() {
        let req = StartAutoRequest {
            start_url: "https://example.com".to_owned(),
            max_pages: 0,
            mark_liked: false,
            mark_viewed: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn auto_request_accepts_valid_input() {
        let req = StartAutoRequest {
            start_url: "https://example.com".to_owned(),
            max_pages: 5,
            mark_liked: true,
            mark_viewed: true,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn update_request_requires_filter() {
        let req = StartUpdateRequest {
            liked_only: false,
            created_after: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn update_request_accepts_liked_only() {
        let req = StartUpdateRequest {
            liked_only: true,
            created_after: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn update_request_accepts_created_after() {
        let req = StartUpdateRequest {
            liked_only: false,
            created_after: Some("2024-01-01".to_owned()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn sse_event_task_id_and_user_id() {
        let event = SseEvent::TaskStarted {
            task_id: 42,
            user_id: "user1".to_owned(),
            task_type: "batch".to_owned(),
        };
        assert_eq!(event.task_id(), 42);
        assert_eq!(event.user_id(), "user1");
        assert!(!event.is_terminal());
        assert_eq!(event.event_type(), "task:started");
    }

    #[test]
    fn sse_event_terminal_detection() {
        let completed = SseEvent::TaskCompleted {
            task_id: 1,
            user_id: "u".to_owned(),
            status: "completed".to_owned(),
            summary: TaskSummary {
                total: 10,
                success: 8,
                failed: 1,
                skipped: 1,
                pages_crawled: 3,
            },
        };
        assert!(completed.is_terminal());

        let cancelled = SseEvent::TaskCancelled {
            task_id: 1,
            user_id: "u".to_owned(),
            summary: TaskSummary {
                total: 10,
                success: 5,
                failed: 0,
                skipped: 0,
                pages_crawled: 1,
            },
        };
        assert!(cancelled.is_terminal());
    }

    #[test]
    fn task_status_from_str_roundtrip() {
        use crate::domains::crawl::domain::model::TaskStatus;
        assert_eq!(TaskStatus::from_str("queued"), Some(TaskStatus::Queued));
        assert_eq!(TaskStatus::from_str("running"), Some(TaskStatus::Running));
        assert_eq!(
            TaskStatus::from_str("completed"),
            Some(TaskStatus::Completed)
        );
        assert_eq!(TaskStatus::from_str("failed"), Some(TaskStatus::Failed));
        assert_eq!(
            TaskStatus::from_str("cancelled"),
            Some(TaskStatus::Cancelled)
        );
        assert_eq!(TaskStatus::from_str("invalid"), None);
    }

    #[test]
    fn task_type_from_str_roundtrip() {
        use crate::domains::crawl::domain::model::TaskType;
        assert_eq!(TaskType::from_str("batch"), Some(TaskType::Batch));
        assert_eq!(TaskType::from_str("auto"), Some(TaskType::Auto));
        assert_eq!(TaskType::from_str("update"), Some(TaskType::Update));
        assert_eq!(TaskType::from_str("invalid"), None);
    }
}
