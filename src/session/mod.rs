use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Idle,
    Running,
    AwaitingInput,
    Done,
    Error,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Idle => write!(f, "IDLE"),
            SessionStatus::Running => write!(f, "RUN"),
            SessionStatus::AwaitingInput => write!(f, "AWAIT"),
            SessionStatus::Done => write!(f, "DONE"),
            SessionStatus::Error => write!(f, "ERR"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProgress {
    pub step: u32,
    pub total_steps: u32,
    pub current_task: String,
    pub etc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub status: SessionStatus,
    pub progress: Option<SessionProgress>,
    pub background_agents: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "chrono::Utc::now")]
    pub status_changed_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn new(name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            name,
            status: SessionStatus::Idle,
            progress: None,
            background_agents: 0,
            created_at: now,
            status_changed_at: now,
        }
    }

    /// Sort priority: AWAIT=0, IDLE=1, RUN=2, DONE=3, ERROR=4
    pub fn sort_key(&self) -> (u8, i64) {
        let priority = match self.status {
            SessionStatus::AwaitingInput => 0,
            SessionStatus::Idle => 1,
            SessionStatus::Running => 2,
            SessionStatus::Done => 3,
            SessionStatus::Error => 4,
        };
        // For AWAIT: sort by most recent first (negative timestamp)
        let time_key = if self.status == SessionStatus::AwaitingInput {
            -self.status_changed_at.timestamp()
        } else {
            0
        };
        (priority, time_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGroup {
    pub name: String,
    pub path: PathBuf,
    pub sessions: Vec<Session>,
    #[serde(skip)]
    pub branch: String,
    #[serde(skip)]
    pub has_open_pr: Option<String>,
    #[serde(skip)]
    pub dirty_files: u32,
    #[serde(skip)]
    pub sub_repos: Vec<SubRepoView>,
}

#[derive(Debug, Clone, Default)]
pub struct SubRepoView {
    pub name: String,
    pub branch: String,
    pub dirty_files: u32,
    pub has_unmerged_pr: bool,
    pub pr_number: String, // e.g. "#466" or ""
}

impl ProjectGroup {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            sessions: Vec::new(),
            branch: String::from("main"),
            has_open_pr: None,
            dirty_files: 0,
            sub_repos: Vec::new(),
        }
    }
}
