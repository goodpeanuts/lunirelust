use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Circuit-breaker threshold for entity-auto-crawl: after this many consecutive
/// non-404 page errors (with no success or empty page in between) the crawl
/// stops and the task is marked failed. This is NOT a page cap; it bounds a
/// sustained source outage. Interspersed content/empty pages reset the counter,
/// so iteration that is making progress is bounded by the eventual 404 instead.
/// Entity-auto-crawl tasks always run with `crawl_task.max_pages = None` (no
/// page cap); this constant is the only backstop. Adjustable; may be promoted
/// to runtime config later.
pub const ENTITY_AUTO_CRAWL_MAX_CONSECUTIVE_ERRORS: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Batch,
    Auto,
    Update,
    Idol,
    EntityAutoCrawl,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Batch => "batch",
            Self::Auto => "auto",
            Self::Update => "update",
            Self::Idol => "idol",
            Self::EntityAutoCrawl => "entity_auto_crawl",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "batch" => Some(Self::Batch),
            "auto" => Some(Self::Auto),
            "update" => Some(Self::Update),
            "idol" => Some(Self::Idol),
            "entity_auto_crawl" => Some(Self::EntityAutoCrawl),
            _ => None,
        }
    }
}

/// The six entity kinds driven by the entity-auto-crawl task type.
///
/// Each exposes a listing `link` and is structurally identical at the data
/// layer (`id`, `name`, `link`, `manual`), living in its own table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityAutoCrawlType {
    Idol,
    Director,
    Label,
    Series,
    Studio,
    Genre,
}

impl EntityAutoCrawlType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idol => "idol",
            Self::Director => "director",
            Self::Label => "label",
            Self::Series => "series",
            Self::Studio => "studio",
            Self::Genre => "genre",
        }
    }

    #[expect(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idol" => Some(Self::Idol),
            "director" => Some(Self::Director),
            "label" => Some(Self::Label),
            "series" => Some(Self::Series),
            "studio" => Some(Self::Studio),
            "genre" => Some(Self::Genre),
            _ => None,
        }
    }

    /// Physical table name for this entity kind. Used to build raw-SQL
    /// selection and summary queries. The enum is trusted (constructed only
    /// from validated input), so interpolating this name carries no injection
    /// risk.
    pub fn table_name(&self) -> &'static str {
        match self {
            Self::Idol => "idol",
            Self::Director => "director",
            Self::Label => "label",
            Self::Series => "series",
            Self::Studio => "studio",
            Self::Genre => "genre",
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

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlTaskInput {
    Batch(BatchTaskInput),
    Auto(AutoTaskInput),
    Update(UpdateTaskInput),
    Idol(IdolTaskInput),
    EntityAutoCrawl(EntityAutoCrawlTaskInput),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskInput {
    pub codes: Vec<String>,
    pub base_url: String,
    pub mark_liked: bool,
    pub mark_viewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTaskInput {
    pub start_url: String,
    pub max_pages: u32,
    pub base_url: String,
    pub mark_liked: bool,
    pub mark_viewed: bool,
    #[serde(default = "default_true")]
    pub append_page_path: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskInput {
    pub filters: UpdateFilters,
    pub target_ids: Vec<String>,
    pub base_url: String,
    #[serde(default)]
    pub update_images: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFilters {
    pub liked_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdolCrawlTarget {
    pub id: i64,
    pub name: String,
    pub link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdolTaskInput {
    pub base_url: String,
    pub idols: Vec<IdolCrawlTarget>,
}

/// Selection scope for an entity-auto-crawl task. Persisted into the task
/// payload so that `cancel_task` can decide whether to undo the claim's round
/// bump (Model C: cancel is scope-aware).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityAutoCrawlScope {
    Uncrawled,
    Failed,
}

impl EntityAutoCrawlScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uncrawled => "uncrawled",
            Self::Failed => "failed",
        }
    }

    #[expect(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "uncrawled" => Some(Self::Uncrawled),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Persisted payload for an entity-auto-crawl task. One task crawls one entity
/// (`{link}/{n}` page by page). `crawl_round` captures the rotation round at
/// selection time and `scope` records how the entity was selected, so cancel
/// and restart-recovery can locate the entity and decide the round effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAutoCrawlTaskInput {
    pub entity_type: EntityAutoCrawlType,
    pub entity_id: i64,
    pub entity_name: String,
    /// Absolute entity listing link, trailing `/` removed. Pagination URL is
    /// built as `{link}/{page}`.
    pub link: String,
    pub base_url: String,
    pub crawl_round: i64,
    pub scope: EntityAutoCrawlScope,
}

#[derive(Debug, Clone)]
pub struct CrawlTaskDetail {
    pub task: CrawlTask,
    pub code_results: Vec<CrawlCodeResult>,
    pub page_results: Vec<CrawlPageResult>,
}
