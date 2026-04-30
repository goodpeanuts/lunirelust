use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Batch,
    Auto,
    Update,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Batch => "batch",
            Self::Auto => "auto",
            Self::Update => "update",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "batch" => Some(Self::Batch),
            "auto" => Some(Self::Auto),
            "update" => Some(Self::Update),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeResultStatus {
    Success,
    Partial,
    Failed,
    Skipped,
}

impl CodeResultStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Self::Success),
            "partial" => Some(Self::Partial),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageResultStatus {
    Processing,
    Success,
    Failed,
}

impl PageResultStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Processing => "processing",
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "processing" => Some(Self::Processing),
            "success" => Some(Self::Success),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrawlTask {
    pub id: i64,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub user_id: String,
    pub mark_liked: bool,
    pub mark_viewed: bool,
    pub input_payload: Option<String>,
    pub max_pages: Option<i32>,
    pub total_codes: i32,
    pub success_count: i32,
    pub fail_count: i32,
    pub skip_count: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct CrawlCodeResult {
    pub id: i64,
    pub task_id: i64,
    pub code: String,
    pub status: CodeResultStatus,
    pub record_id: Option<String>,
    pub images_downloaded: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CrawlPageResult {
    pub id: i64,
    pub task_id: i64,
    pub page_number: i32,
    pub status: PageResultStatus,
    pub records_found: i32,
    pub records_crawled: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Serializable input payloads for restart recovery

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlTaskInput {
    Batch(BatchTaskInput),
    Auto(AutoTaskInput),
    Update(UpdateTaskInput),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskInput {
    pub codes: Vec<String>,
    pub mark_liked: bool,
    pub mark_viewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTaskInput {
    pub start_url: String,
    pub max_pages: u32,
    pub mark_liked: bool,
    pub mark_viewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskInput {
    pub filters: UpdateFilters,
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFilters {
    pub liked_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CrawlTaskDetail {
    pub task: CrawlTask,
    pub code_results: Vec<CrawlCodeResult>,
    pub page_results: Vec<CrawlPageResult>,
}
