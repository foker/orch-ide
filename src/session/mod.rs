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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionColor {
    Grey, Red, Orange, Yellow, Green, Blue, Purple, Pink,
}

impl Default for SessionColor {
    fn default() -> Self { SessionColor::Grey }
}

impl SessionColor {
    pub fn all() -> &'static [SessionColor] {
        &[SessionColor::Grey, SessionColor::Red, SessionColor::Orange, SessionColor::Yellow,
          SessionColor::Green, SessionColor::Blue, SessionColor::Purple, SessionColor::Pink]
    }

    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            SessionColor::Grey => (0x88, 0x88, 0x99),
            SessionColor::Red => (0xf0, 0x50, 0x50),
            SessionColor::Orange => (0xf0, 0x80, 0x40),
            SessionColor::Yellow => (0xf0, 0xc0, 0x50),
            SessionColor::Green => (0x3d, 0xd6, 0x8c),
            SessionColor::Blue => (0x50, 0x90, 0xf0),
            SessionColor::Purple => (0x90, 0x70, 0xf0),
            SessionColor::Pink => (0xf0, 0x70, 0xc0),
        }
    }
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
    #[serde(default)]
    pub color: SessionColor,
    /// Pinned sessions float to a dedicated zone at the top of the sidebar.
    #[serde(default)]
    pub pinned: bool,
    /// Per-session agent backend override (AgentBackend::as_str). None = use the
    /// global backend from settings.
    #[serde(default)]
    pub backend_override: Option<String>,
    #[serde(default)]
    pub pipeline_run: Option<PipelineRun>,
    /// Set by Tick from the Stop hook payload. Pipeline driver gates auto-advance
    /// on this — claude ending its last turn with `?` means it's asking the user
    /// something, not finishing work.
    #[serde(skip, default)]
    pub last_was_question: bool,
    /// True when the running step emitted the `ORCHIDE: DONE` completion marker.
    #[serde(skip, default)]
    pub orchide_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineStep {
    pub prompt: String,
    #[serde(default)]
    pub interactive: bool,
    /// Per-step agent backend (AgentBackend::as_str). None = global settings backend.
    #[serde(default)]
    pub backend: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub steps: Vec<PipelineStep>,
    /// Prepended to every step's prompt. `{requirements}` is replaced with the
    /// absolute path to the project's `.orchpipeline` file at run time.
    #[serde(default = "default_prepend_template")]
    pub prepend_template: String,
    /// Appended to every step's prompt (after the step text). Seeded with the
    /// pipeline-meta scaffolding so the whole prompt is visible/editable.
    #[serde(default = "default_append_template")]
    pub append_template: String,
}

pub fn default_prepend_template() -> String {
    "The main requirements content is in this file: {requirements}".to_string()
}

/// Default text appended to every step's prompt. Previously this was baked into
/// the code (invisible); it now lives in the editable append template so the
/// full prompt is visible. Tokens {step_n} {total} {summaries_dir}
/// {summary_path} {requirements} are expanded at run time.
pub fn default_append_template() -> String {
    r#"--- pipeline meta (managed by orch-ide, do not skip) ---
You are running step {step_n} of {total} in an automated pipeline.
1. BEFORE starting work: list and read every existing summary in `{summaries_dir}/` (files named `step-*.md`). They contain hand-offs from prior steps.
2. AFTER finishing work for this step (and BEFORE returning control), write a short summary to `{summary_path}` with this format:
```markdown
# Step {step_n} summary
## What was done
- <3-6 bullets>
## Key files / commits / branches
- <paths, commit shas, PR links>
## Open questions / next-step pointers
- <what the next step needs to know — gotchas, scope edges, things you intentionally did not do>
```
Create the directory if it does not exist (`mkdir -p {summaries_dir}`). Keep the summary terse — it is read by other claude steps, not by humans.
"#.to_string() + COMPLETION_MARKER_NOTE
}

/// The completion-marker instruction. The pipeline driver auto-advances a
/// non-interactive step ONLY when claude's final message contains `ORCHIDE: DONE`.
/// Kept separate so old pipelines can be migrated to include it.
pub const COMPLETION_MARKER_NOTE: &str = "3. COMPLETION MARKER: ONLY when this step is genuinely finished (all the work done and the summary written), end your FINAL message with this exact line on its own:\nORCHIDE: DONE\nDo NOT emit `ORCHIDE: DONE` if any work remains, if you are asking the user a question, or if you are handing off mid-task. The pipeline auto-advances to the next step ONLY when it sees this marker — if you stop without it, the pipeline waits for you instead of skipping ahead.";

impl PipelineDef {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            name,
            steps: Vec::new(),
            prepend_template: default_prepend_template(),
            append_template: default_append_template(),
        }
    }
}

/// Live pipeline run tied to a Session. Persisted across restarts; transient
/// fields (last_seen_running / prompt_pending_send / send_delay_ticks) are reset
/// on load since the underlying PTY dies with the app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub pipeline_name: String,
    pub steps: Vec<PipelineStep>,
    pub current_step: usize,
    pub requirements_path: PathBuf,
    /// Snapshot of the pipeline's prepend template at run start — keeps the run
    /// stable even if the user edits the pipeline definition mid-run.
    #[serde(default = "default_prepend_template")]
    pub prepend_template: String,
    #[serde(default = "default_append_template")]
    pub append_template: String,
    /// Unix seconds when the current step was (re)spawned. A summary file for the
    /// current step that is newer than this means the step finished — a robust
    /// completion signal that survives restarts and a missed marker file.
    #[serde(default)]
    pub step_spawn_at: i64,
    /// Per-step agent session id (claude conversation id, captured by the Stop
    /// hook) so re-opening a step resumes its conversation instead of starting
    /// fresh. Parallel to `steps`; `None` = never captured / start fresh. Cleared
    /// when the run starts with "Start fresh".
    #[serde(default)]
    pub step_sessions: Vec<Option<String>>,
    #[serde(skip, default)]
    pub last_seen_running: bool,
    #[serde(skip, default)]
    pub prompt_pending_send: bool,
    #[serde(skip, default)]
    pub send_delay_ticks: u8,
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
            color: SessionColor::Grey,
            pinned: false,
            backend_override: None,
            pipeline_run: None,
            last_was_question: false,
            orchide_done: false,
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
pub struct DeploymentInfo {
    pub env: String,
    pub state: String,
    pub url: String,
}

#[derive(Debug, Clone, Default)]
pub struct DevServerInfo {
    pub url: String,
    pub running: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SubRepoView {
    pub name: String,
    pub branch: String,
    pub commit_short: String,
    pub dirty_files: u32,
    pub unpushed_commits: u32,
    pub has_unmerged_pr: bool,
    pub pr_number: String,
    pub pr_url: String,
    pub deployments: Vec<DeploymentInfo>,
    pub dev_servers: Vec<DevServerInfo>,
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
