mod session;
mod notion;
mod explorer;
mod git_info;
mod hooks;
mod persistence;
mod logging;
mod voice;

use iced::widget::{
    button, column, container, row, scrollable, text,
    text_editor, text_input, mouse_area, Column, Row, Space, rule,
};
use iced::{Element, Fill, Font, Padding, Subscription, Theme, Color, Border, Background, Task};
use session::{PipelineDef, PipelineRun, PipelineStep, ProjectGroup, Session, SessionStatus};
use std::path::PathBuf;
use std::time::Duration;

const MONO_FONT: Font = Font::with_name("JetBrains Mono");
const SESSION_INPUT_ID: &str = "session-name-input";
const APP_VERSION: &str = "0.1.7";

fn main() -> iced::Result {
    logging::init();
    bootstrap_path();
    iced::application(App::boot, App::update, App::view)
        .title("Claude Sessions")
        .theme(App::theme)
        .subscription(App::subscription)
        .window_size((1200.0, 800.0))
        .run()
}

/// When the .app is launched from Finder/Spotlight, the process inherits launchd's
/// stripped PATH (`/usr/bin:/bin:/usr/sbin:/sbin`) — Homebrew/nvm bins are missing.
/// Resolve the user's login-shell PATH once and pin it on the process so every
/// child `Command` (gh, git, claude, …) sees the same PATH iTerm would.
fn bootstrap_path() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let Ok(out) = std::process::Command::new(&shell)
        .args(["-lc", "echo -n $PATH"])
        .output()
    else {
        app_log!("bootstrap_path: failed to spawn login shell");
        return;
    };
    if !out.status.success() {
        app_log!("bootstrap_path: login shell exit={}", out.status);
        return;
    }
    let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if resolved.is_empty() {
        app_log!("bootstrap_path: empty PATH from login shell");
        return;
    }
    let resolved = prepend_local_bin(resolved);
    app_log!("bootstrap_path: PATH={}", resolved);
    // SAFETY: main() is single-threaded at this point — iced hasn't started yet.
    unsafe { std::env::set_var("PATH", &resolved); }
}

// ─── Theme System ───

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppTheme { Midnight, VsCode, Darcula, GitHub, Monokai, Catppuccin }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen { Ide, Board }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentBackend { ClaudeCode, OpenCode, Codex, XiaomiMimo }

/// Model id the Xiaomi MiMo backend pins opencode to (provider/model).
const MIMO_MODEL: &str = "mimo/mimo-v2.5-pro";

impl AgentBackend {
    fn all() -> &'static [AgentBackend] {
        &[AgentBackend::ClaudeCode, AgentBackend::OpenCode, AgentBackend::Codex, AgentBackend::XiaomiMimo]
    }
    fn label(&self) -> &'static str {
        match self {
            AgentBackend::ClaudeCode => "Claude Code",
            AgentBackend::OpenCode => "OpenCode",
            AgentBackend::Codex => "Codex",
            AgentBackend::XiaomiMimo => "Xiaomi MiMo",
        }
    }
    fn from_str(s: &str) -> Self {
        match s {
            "OpenCode" => AgentBackend::OpenCode,
            "Codex" => AgentBackend::Codex,
            "XiaomiMimo" => AgentBackend::XiaomiMimo,
            _ => AgentBackend::ClaudeCode,
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            AgentBackend::ClaudeCode => "ClaudeCode",
            AgentBackend::OpenCode => "OpenCode",
            AgentBackend::Codex => "Codex",
            AgentBackend::XiaomiMimo => "XiaomiMimo",
        }
    }
}

impl AppTheme {
    fn all() -> &'static [AppTheme] {
        &[AppTheme::Midnight, AppTheme::VsCode, AppTheme::Darcula, AppTheme::GitHub, AppTheme::Monokai, AppTheme::Catppuccin]
    }
    fn name(&self) -> &'static str {
        match self {
            AppTheme::Midnight => "Midnight",
            AppTheme::VsCode => "VS Code Dark+",
            AppTheme::Darcula => "Darcula (JetBrains)",
            AppTheme::GitHub => "GitHub Dark",
            AppTheme::Monokai => "Monokai Pro",
            AppTheme::Catppuccin => "Catppuccin Mocha",
        }
    }
    fn colors(&self) -> TC {
        match self {
            AppTheme::Midnight => TC {
                bg_deep: c(0x0a, 0x0a, 0x0f), bg_panel: c(0x11, 0x11, 0x18),
                bg_card: c(0x1a, 0x1a, 0x24), bg_card_hover: c(0x22, 0x22, 0x3a),
                bg_terminal: c(0x0d, 0x0d, 0x14), border: c(0x2a, 0x2a, 0x3a),
                border_active: c(0x4a, 0x4a, 0x6a),
                text_primary: c(0xe8, 0xe8, 0xf0), text_secondary: c(0x88, 0x88, 0xa8),
                text_muted: c(0x55, 0x55, 0x70),
                green: c(0x3d, 0xd6, 0x8c), yellow: c(0xf0, 0xc0, 0x50),
                blue: c(0x50, 0x90, 0xf0), red: c(0xf0, 0x50, 0x50),
                purple: c(0x90, 0x70, 0xf0), orange: c(0xf0, 0x80, 0x40),
            },
            AppTheme::VsCode => TC {
                bg_deep: c(0x1e, 0x1e, 0x1e), bg_panel: c(0x25, 0x25, 0x26),
                bg_card: c(0x2d, 0x2d, 0x2d), bg_card_hover: c(0x37, 0x37, 0x3d),
                bg_terminal: c(0x1e, 0x1e, 0x1e), border: c(0x3c, 0x3c, 0x3c),
                border_active: c(0x00, 0x7a, 0xcc),
                text_primary: c(0xd4, 0xd4, 0xd4), text_secondary: c(0x9d, 0x9d, 0x9d),
                text_muted: c(0x6a, 0x6a, 0x6a),
                green: c(0x6a, 0x99, 0x55), yellow: c(0xdc, 0xdc, 0xaa),
                blue: c(0x56, 0x9c, 0xd6), red: c(0xf4, 0x47, 0x47),
                purple: c(0xc5, 0x86, 0xc0), orange: c(0xce, 0x91, 0x78),
            },
            AppTheme::Darcula => TC {
                bg_deep: c(0x2b, 0x2b, 0x2b), bg_panel: c(0x3c, 0x3f, 0x41),
                bg_card: c(0x45, 0x49, 0x4a), bg_card_hover: c(0x4e, 0x52, 0x54),
                bg_terminal: c(0x2b, 0x2b, 0x2b), border: c(0x51, 0x51, 0x51),
                border_active: c(0x4a, 0x88, 0xc7),
                text_primary: c(0xa9, 0xb7, 0xc6), text_secondary: c(0x80, 0x80, 0x80),
                text_muted: c(0x60, 0x63, 0x66),
                green: c(0x6a, 0x87, 0x59), yellow: c(0xff, 0xc6, 0x6d),
                blue: c(0x68, 0x97, 0xbb), red: c(0xff, 0x6b, 0x68),
                purple: c(0x98, 0x76, 0xaa), orange: c(0xcc, 0x78, 0x32),
            },
            AppTheme::GitHub => TC {
                bg_deep: c(0x0d, 0x11, 0x17), bg_panel: c(0x16, 0x1b, 0x22),
                bg_card: c(0x21, 0x26, 0x2d), bg_card_hover: c(0x29, 0x2e, 0x36),
                bg_terminal: c(0x0d, 0x11, 0x17), border: c(0x30, 0x36, 0x3d),
                border_active: c(0x58, 0xa6, 0xff),
                text_primary: c(0xc9, 0xd1, 0xd9), text_secondary: c(0x8b, 0x94, 0x9e),
                text_muted: c(0x6e, 0x76, 0x81),
                green: c(0x3f, 0xb9, 0x50), yellow: c(0xd2, 0x99, 0x22),
                blue: c(0x58, 0xa6, 0xff), red: c(0xf8, 0x51, 0x49),
                purple: c(0xbc, 0x8c, 0xff), orange: c(0xf0, 0x88, 0x3e),
            },
            AppTheme::Monokai => TC {
                bg_deep: c(0x2d, 0x2a, 0x2e), bg_panel: c(0x36, 0x33, 0x37),
                bg_card: c(0x40, 0x3e, 0x41), bg_card_hover: c(0x4a, 0x47, 0x4b),
                bg_terminal: c(0x2d, 0x2a, 0x2e), border: c(0x4a, 0x47, 0x4b),
                border_active: c(0xff, 0xd8, 0x66),
                text_primary: c(0xfc, 0xfc, 0xfa), text_secondary: c(0xc1, 0xc0, 0xc0),
                text_muted: c(0x72, 0x70, 0x72),
                green: c(0xa9, 0xdc, 0x76), yellow: c(0xff, 0xd8, 0x66),
                blue: c(0x78, 0xdc, 0xe8), red: c(0xff, 0x61, 0x88),
                purple: c(0xab, 0x9d, 0xf2), orange: c(0xfc, 0x98, 0x67),
            },
            AppTheme::Catppuccin => TC {
                bg_deep: c(0x1e, 0x1e, 0x2e), bg_panel: c(0x24, 0x24, 0x3e),
                bg_card: c(0x31, 0x32, 0x44), bg_card_hover: c(0x3b, 0x3b, 0x52),
                bg_terminal: c(0x1e, 0x1e, 0x2e), border: c(0x45, 0x47, 0x5a),
                border_active: c(0x89, 0xb4, 0xfa),
                text_primary: c(0xcd, 0xd6, 0xf4), text_secondary: c(0xa6, 0xad, 0xc8),
                text_muted: c(0x6c, 0x70, 0x86),
                green: c(0xa6, 0xe3, 0xa1), yellow: c(0xf9, 0xe2, 0xaf),
                blue: c(0x89, 0xb4, 0xfa), red: c(0xf3, 0x8b, 0xa8),
                purple: c(0xcb, 0xa6, 0xf7), orange: c(0xfa, 0xb3, 0x87),
            },
        }
    }
}

fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// Map a Notion option color name to an RGB color (approximating Notion's palette).
fn notion_color(name: &str) -> Color {
    match name {
        "gray" | "grey" => c(0x9b, 0xa1, 0xa6),
        "brown" => c(0xa6, 0x7c, 0x5b),
        "orange" => c(0xe8, 0x8a, 0x3d),
        "yellow" => c(0xd9, 0xb5, 0x3d),
        "green" => c(0x4d, 0xab, 0x6e),
        "blue" => c(0x4d, 0x90, 0xcf),
        "purple" => c(0x9a, 0x6d, 0xd7),
        "pink" => c(0xd1, 0x5b, 0x9e),
        "red" => c(0xe0, 0x5b, 0x52),
        _ => c(0x9b, 0xa1, 0xa6), // default
    }
}

#[derive(Clone)]
struct TC {
    bg_deep: Color, bg_panel: Color, bg_card: Color, bg_card_hover: Color,
    bg_terminal: Color, border: Color, border_active: Color,
    text_primary: Color, text_secondary: Color, text_muted: Color,
    green: Color, yellow: Color, blue: Color, red: Color, purple: Color, orange: Color,
}

impl TC {
    fn status_color(&self, status: &SessionStatus) -> Color {
        match status {
            SessionStatus::Running => self.yellow,
            SessionStatus::AwaitingInput => self.green,
            SessionStatus::Done => self.green,
            SessionStatus::Error => self.red,
            SessionStatus::Idle => self.text_muted,
        }
    }
}

// ─── Messages ───

#[derive(Debug, Clone)]
enum Message {
    OpenProject, ProjectPicked(Option<PathBuf>),
    NewProject, NewProjectFolderPicked(Option<PathBuf>),
    AddSession(usize), NewSessionInCurrentFolder,
    SessionNameSubmit(usize, String), SessionNameChanged(String),
    CancelSessionDialog,
    ToggleLaunchClaude,
    SelectSession(usize, usize), KillSession(usize, usize), MakeIdle(usize, usize),
    CardHover(Option<(usize, usize)>),
    StartRenameSession(usize, usize), RenameSessionInput(String), RenameSessionSubmit,
    SetSessionColor(usize, usize, session::SessionColor),
    SetNewSessionColor(session::SessionColor),
    RemoveProject(usize), DeleteProjectDir(usize), ConfirmDeleteDir(usize), CancelDelete, ToggleProjectExpand(usize),
    OpenFile(PathBuf),
    // Async git info
    RefreshProject(usize), // refresh git + PR + deployments for one project
    FetchGitInfo(usize),
    GitInfoFetched(usize, Option<git_info::GitInfo>),
    // Deployments
    FetchDeployments(usize), // project index
    DeploymentsFetched(usize, Vec<(String, Vec<(String, String, String)>)>), // pi, vec of (sub_repo_name, deployments)
    ToggleDeploymentDropdown,
    OpenUrl(String),
    // Voice
    VoiceToggle, VoiceResult(Result<String, String>),
    GroqKeyChanged(String),
    ToggleDangerouslySkipPermissions,
    ToggleDatePrefix,
    // Quick prompts
    SendQuickPrompt(String),
    AddQuickPrompt, RemoveQuickPrompt(usize), QuickPromptInput(String),
    UpdateCheckResult(Option<String>), // Some("0.2.0") if newer version available
    DismissUpdate,
    TerminalExited(usize, usize), // pi, si — terminal gracefully stopped, now remove
    ProjectTerminalsExited(usize), // pi — all terminals stopped, now move to trash in async
    ProjectDirTrashed(usize, Result<(), String>), // pi, result of trash operation
    ResizeSidebar(f32),
    ToggleFileExpand(usize), RefreshExplorer, RefreshAll, Tick, Blink, FrameProbe,
    KeyboardEvent(iced::keyboard::Event),
    ToggleSettings, SetTheme(AppTheme),
    TermEvent(iced_term::Event),
    // Agent backend + screen
    SetAgentBackend(AgentBackend),
    SetScreen(Screen),
    // Notion
    NotionTokenChanged(String),
    NotionConnect,
    NotionDatabasesFetched(Result<Vec<notion::NotionDatabase>, String>),
    NotionSelectDatabase(String),
    NotionSchemaFetched(Result<notion::NotionDatabase, String>),
    NotionTasksFetched(Result<Vec<notion::NotionTask>, String>),
    NotionRefresh,
    NotionSetGroupBy(String),
    BoardAddTaskStart(String),
    BoardAddTaskInput(String),
    BoardAddTaskSubmit,
    BoardAddTaskCancel,
    BoardTaskCreated(Result<(), String>),
    NotionOpenTask(String),
    OpenTaskBoard(String),
    NotionBodyFetched(String, Result<Vec<notion::Block>, String>),
    NotionCommentsFetched(String, Result<Vec<notion::Comment>, String>),
    CommentInput(String),
    PostComment,
    CommentPosted(Result<(), String>),
    NotionCloseTask,
    TaskOpenDir,
    TaskNewDir,
    TaskDirPicked(Option<PathBuf>),
    TaskNewParentPicked(Option<PathBuf>),
    SetNewSessionBackend(AgentBackend),
    ReopenTaskLink(usize),
    // Pipelines — settings editor
    AddPipeline,
    RemovePipeline(usize),
    PipelineNameChanged(usize, String),
    PipelinePrependChanged(usize, String),
    AddPipelineStep(usize),
    RemovePipelineStep(usize, usize),
    MovePipelineStep(usize, usize, isize),
    PipelineStepPromptChanged(usize, usize, String),
    TogglePipelineStepInteractive(usize, usize),
    CyclePipelineStepBackend(usize, usize),
    ToggleEditPipeline(usize),
    // Pipelines — run modal
    OpenRunPipelineModal(usize, usize),
    ClosePipelineModal,
    SelectPipelineForRun(usize),
    PipelineRequirementsAction(text_editor::Action),
    StartPipeline,
    // Pipelines — execution
    PipelineSpawnNext(usize, usize),
    PipelineSendPrompt(usize, usize),
    PipelineSubmitPrompt(usize, usize),
    // Pipelines — toolbar controls
    PipelineNextStep(usize, usize),
    PipelineJumpToStep(usize, usize, usize),
    PipelineStop(usize, usize),
}

// ─── App ───

struct TerminalInstance { terminal: iced_term::Terminal, id: u64 }

struct App {
    projects: Vec<ProjectGroup>,
    active_project: Option<usize>,
    active_session: Option<(usize, usize)>,
    file_entries: Vec<explorer::FileEntry>,
    terminals: Vec<((usize, usize), TerminalInstance)>,
    next_term_id: u64,
    session_name_input: String,
    show_session_dialog: Option<usize>,
    new_project_parent: Option<PathBuf>, // parent folder for "+" new project flow
    launch_agent: bool,
    agent_backend: AgentBackend,
    notion_token: String,
    notion_database_id: Option<String>,
    notion_group_by_prop: Option<String>,
    // Notion runtime (not persisted)
    notion_databases: Vec<notion::NotionDatabase>,
    notion_schema: Option<notion::NotionDatabase>,
    notion_tasks: Vec<notion::NotionTask>,
    notion_error: Option<String>,
    notion_loading: bool,
    adding_task_col: Option<String>,
    new_task_title: String,
    creating_task: bool,
    screen: Screen,
    open_task: Option<String>,
    open_task_body: Vec<notion::Block>,
    open_task_body_loading: bool,
    /// Backend chosen in the task detail for the next session created from it
    /// (defaults to the global settings backend when a task is opened).
    new_session_backend: AgentBackend,
    open_task_comments: Vec<notion::Comment>,
    comments_loading: bool,
    comment_input: String,
    comment_posting: bool,
    task_links: std::collections::HashMap<String, Vec<persistence::TaskLink>>,
    show_settings: bool,
    confirm_delete: Option<(usize, Vec<String>)>,
    renaming_session: Option<(usize, usize)>,
    rename_input: String,
    new_session_color: session::SessionColor,
    // Deployments
    show_deployment_dropdown: bool,
    dangerously_skip_permissions: bool,
    quick_prompts: Vec<String>,
    quick_prompt_input: String,
    update_available: Option<String>,
    // Voice
    voice_recorder: voice::AudioRecorder,
    groq_api_key: String,
    voice_transcribing: bool,
    current_theme: AppTheme,
    sidebar_width: f32,
    date_prefix_enabled: bool,
    hovered_card: Option<(usize, usize)>,
    tick_count: u32,
    blink_on: bool,
    // Tracks which projects currently have git/deploy fetches in flight.
    // Project is considered "refreshing" while either set contains its index.
    refresh_pending_git: std::collections::HashSet<usize>,
    refresh_pending_deps: std::collections::HashSet<usize>,
    // Last FrameProbe tick for measuring UI loop jitter (lag spikes show as
    // delta > probe interval).
    last_frame_probe: Option<std::time::Instant>,
    // Pipelines
    pipelines: Vec<PipelineDef>,
    editing_pipeline: Option<usize>,
    // Active "Run pipeline" modal targeting (pi, si).
    show_pipeline_modal: Option<(usize, usize)>,
    pipeline_modal_selected: Option<usize>,
    pipeline_modal_requirements: text_editor::Content,
}

impl Default for App {
    fn default() -> Self {
        Self {
            projects: Vec::new(), active_project: None, active_session: None,
            file_entries: Vec::new(), terminals: Vec::new(), next_term_id: 0,
            session_name_input: String::new(), show_session_dialog: None, new_project_parent: None,
            launch_agent: true,
            agent_backend: AgentBackend::ClaudeCode,
            notion_token: String::new(), notion_database_id: None, notion_group_by_prop: None,
            notion_databases: Vec::new(), notion_schema: None, notion_tasks: Vec::new(),
            notion_error: None, notion_loading: false,
            adding_task_col: None, new_task_title: String::new(), creating_task: false,
            screen: Screen::Ide, open_task: None, open_task_body: Vec::new(), open_task_body_loading: false,
            new_session_backend: AgentBackend::ClaudeCode,
            open_task_comments: Vec::new(), comments_loading: false, comment_input: String::new(), comment_posting: false,
            task_links: std::collections::HashMap::new(),
            show_settings: false, confirm_delete: None, renaming_session: None, rename_input: String::new(), new_session_color: session::SessionColor::Grey,
            show_deployment_dropdown: false,
            dangerously_skip_permissions: true, quick_prompts: Vec::new(), quick_prompt_input: String::new(), update_available: None,
            voice_recorder: voice::AudioRecorder::new(), groq_api_key: String::new(), voice_transcribing: false,
            current_theme: AppTheme::Midnight, sidebar_width: 280.0, date_prefix_enabled: true, hovered_card: None, tick_count: 0, blink_on: true,
            refresh_pending_git: std::collections::HashSet::new(), refresh_pending_deps: std::collections::HashSet::new(),
            last_frame_probe: None,
            pipelines: Vec::new(),
            editing_pipeline: None,
            show_pipeline_modal: None,
            pipeline_modal_selected: None,
            pipeline_modal_requirements: text_editor::Content::new(),
        }
    }
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let mut app = Self::default();
        // Load persisted state
        if let Some(state) = persistence::load() {
            app.projects = state.projects;
            // Prune garbage ProjectGroups that have no sessions. Sidebar already
            // hides them (no header is rendered for sessionless projects, see
            // view_sessions), so they were invisible-but-persistent — every
            // 📁 Open + Cancel left one behind, eventually piling up to ~90+
            // entries that all got git+gh-fetched on boot. Pruning here is safe
            // because the user has no way to interact with them in the UI.
            let before = app.projects.len();
            app.projects.retain(|p| !p.sessions.is_empty());
            let pruned = before - app.projects.len();
            if pruned > 0 {
                app_log!("boot: pruned {} sessionless project(s) from state", pruned);
            }
            // Reset all session statuses to Idle (terminals are dead)
            for p in &mut app.projects {
                for s in &mut p.sessions {
                    s.status = session::SessionStatus::Idle;
                    s.progress = None;
                }
            }
            app.current_theme = match state.theme.as_str() {
                "VsCode" => AppTheme::VsCode,
                "Darcula" => AppTheme::Darcula,
                "GitHub" => AppTheme::GitHub,
                "Monokai" => AppTheme::Monokai,
                "Catppuccin" => AppTheme::Catppuccin,
                _ => AppTheme::Midnight,
            };
            app.sidebar_width = state.sidebar_width;
            app.groq_api_key = state.groq_api_key;
            app.dangerously_skip_permissions = state.dangerously_skip_permissions;
            app.quick_prompts = state.quick_prompts;
            app.date_prefix_enabled = state.date_prefix_enabled;
            app.notion_token = state.notion_token;
            app.notion_database_id = state.notion_database_id;
            app.notion_group_by_prop = state.notion_group_by_prop;
            app.agent_backend = AgentBackend::from_str(&state.agent_backend);
            app.task_links = state.task_links;
            app.pipelines = state.pipelines;
            // Git info loaded lazily on SelectSession (no blocking boot)
        }
        // Show settings if no projects yet
        if app.projects.is_empty() {
            app.show_settings = true;
        }
        // Check for updates async
        let check_update = Task::perform(
            async { check_latest_version().await },
            Message::UpdateCheckResult,
        );
        // Populate sub-repo / PR / deployment info on boot — but ONLY for projects
        // that actually have sessions. Empty/abandoned projects (e.g. opened folder
        // and never created a session) stay dormant: no `gh` API calls, no `git`
        // subprocesses, no GitInfoFetched message storm hammering the UI thread.
        // They refresh on demand: SelectSession, RefreshProject, or RefreshAll.
        //
        // Before this filter every cold-start fired N=projects.len() parallel
        // git+gh tasks; with 95 stale entries that was a multi-second UI freeze.
        let mut boot_tasks: Vec<Task<Message>> = vec![check_update];
        for pi in 0..app.projects.len() {
            if app.projects[pi].sessions.is_empty() {
                continue;
            }
            let path = app.projects[pi].path.clone();
            boot_tasks.push(Task::perform(
                // spawn_blocking: get_git_info_with_pr shells out to git/gh (blocking).
                // Running it directly on the async executor starves every other
                // boot task (incl. the Notion schema fetch) for tens of seconds.
                async move {
                    tokio::task::spawn_blocking(move || git_info::get_git_info_with_pr(&path))
                        .await.ok().flatten()
                },
                move |info| Message::GitInfoFetched(pi, info),
            ));
        }
        // Auto-load the Notion board when token + db are persisted
        app_log!("boot notion: token_empty={} db_id={:?}", app.notion_token.is_empty(), app.notion_database_id);
        if !app.notion_token.is_empty() {
            if let Some(db_id) = app.notion_database_id.clone() {
                let tok = app.notion_token.clone();
                app_log!("boot notion: firing fetch_schema for {}", db_id);
                boot_tasks.push(Task::perform(notion::fetch_schema(tok, db_id), Message::NotionSchemaFetched));
            }
        }
        (app, Task::batch(boot_tasks))
    }
    fn tc(&self) -> TC { self.current_theme.colors() }

    fn spawn_session_terminal(&mut self, pi: usize, si: usize, resume: bool) {
        let session_name = self.projects[pi].sessions[si].name.clone();
        // Determine program and args (multi-backend: claude / opencode / codex / mimo).
        // Per-session override wins over the global settings backend.
        let backend = self.projects[pi].sessions[si].backend_override.as_deref()
            .map(AgentBackend::from_str)
            .unwrap_or(self.agent_backend);
        let claude_path = which_claude();
        let opencode_path = which_opencode();
        let codex_path = which_codex();
        let (program, args) = build_agent_command(
            backend,
            self.launch_agent,
            resume,
            &claude_path,
            &opencode_path,
            &codex_path,
            &session_name,
            self.dangerously_skip_permissions,
        );
        self.spawn_terminal_with(pi, si, program, args);
    }

    fn spawn_terminal_with(&mut self, pi: usize, si: usize, program: String, args: Vec<String>) {
        app_log!("spawn_terminal: pi={} si={} program={} args={:?} (active terminals: {})", pi, si, program, args, self.terminals.len());
        let cwd = self.projects[pi].path.clone();
        let tid = self.next_term_id;
        self.next_term_id += 1;

        let mut env = std::collections::HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        // Get full PATH from user's login shell (preserves exact order from .zprofile/.zshrc)
        let user_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let full_path = std::process::Command::new(&user_shell)
            .args(["-lc", "echo $PATH"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
        // Prepend $HOME/.local/bin so native claude install check is satisfied
        // (claude v2+ warns "Native installation exists but ~/.local/bin is not in your PATH"
        //  if it doesn't see this entry in process.env.PATH, even when claude itself runs from there)
        let full_path = prepend_local_bin(full_path);
        app_log!("spawn_terminal: PATH={}", full_path);
        env.insert("PATH".to_string(), full_path);

        let settings = iced_term::settings::Settings {
            backend: iced_term::settings::BackendSettings {
                program,
                args,
                working_directory: Some(cwd),
                env,
                ..Default::default()
            },
            ..Default::default()
        };

        if let Ok(terminal) = iced_term::Terminal::new(tid, settings) {
            app_log!("  terminal created tid={}", tid);
            // Setup hooks — only for Claude (OpenCode does not emit them)
            if self.agent_backend == AgentBackend::ClaudeCode && self.launch_agent {
                let sid = &self.projects[pi].sessions[si].id;
                if let Ok(hp) = hooks::create_hook_script(sid) {
                    let _ = hooks::configure_claude_hooks(&self.projects[pi].path, &hp);
                }
            }
            self.terminals.push(((pi, si), TerminalInstance { terminal, id: tid }));
        }
    }

    /// Build the full prompt for a pipeline step.
    /// `{requirements}` in the template is substituted with the absolute path
    /// to the project's `.orchpipeline` file. The user-prompt is bracketed by
    /// (a) the user-editable prepend template and
    /// (b) a manager-baked suffix that instructs claude to read prior step
    ///     summaries and write its own at the end.
    fn build_step_prompt(
        step_prompt: &str,
        requirements_path: &std::path::Path,
        prepend_template: &str,
        project_path: &std::path::Path,
        step_idx: usize,
        total_steps: usize,
    ) -> String {
        let prepend = prepend_template.replace("{requirements}", &requirements_path.display().to_string());
        let summaries_dir = project_path.join(".orchpipeline-summaries");
        let summary_file = summaries_dir.join(format!("step-{}.md", step_idx + 1));
        let suffix = format!(
            "\n\n--- pipeline meta (managed by orch-ide, do not skip) ---\n\
             You are running step {step_n} of {total} in an automated pipeline.\n\
             1. BEFORE starting work: list and read every existing summary in `{summaries_dir}/` (files named `step-*.md`). They contain hand-offs from prior steps.\n\
             2. AFTER finishing work for this step (and BEFORE returning control), write a short summary to `{summary_path}` with this format:\n\
             ```markdown\n\
             # Step {step_n} summary\n\
             ## What was done\n\
             - <3-6 bullets>\n\
             ## Key files / commits / branches\n\
             - <paths, commit shas, PR links>\n\
             ## Open questions / next-step pointers\n\
             - <what the next step needs to know — gotchas, scope edges, things you intentionally did not do>\n\
             ```\n\
             Create the directory if it does not exist (`mkdir -p {summaries_dir}`). Keep the summary terse — it is read by other claude steps, not by humans.",
            step_n = step_idx + 1,
            total = total_steps,
            summaries_dir = summaries_dir.display(),
            summary_path = summary_file.display(),
        );
        let body = if prepend.trim().is_empty() {
            step_prompt.to_string()
        } else {
            format!("{}\n\n{}", prepend, step_prompt)
        };
        format!("{}{}", body, suffix)
    }

    /// Spawn a terminal for the current step of an in-flight pipeline.
    /// Non-interactive: claude -p "<prompt>" — runs once and exits.
    /// Interactive: claude TUI; the prompt is typed in by a follow-up message.
    fn spawn_pipeline_step(&mut self, pi: usize, si: usize) -> Task<Message> {
        let Some(run) = self.projects[pi].sessions[si].pipeline_run.as_ref() else {
            return Task::none();
        };
        let Some(step) = run.steps.get(run.current_step).cloned() else {
            return Task::none();
        };
        let req_path = run.requirements_path.clone();
        let prepend = run.prepend_template.clone();
        let step_idx = run.current_step;
        let total = run.steps.len();
        let project_path = self.projects[pi].path.clone();
        let full_prompt = Self::build_step_prompt(&step.prompt, &req_path, &prepend, &project_path, step_idx, total);
        let claude_path = which_claude();

        // Reset hook status so we observe a fresh Running → AwaitingInput edge.
        let sid = self.projects[pi].sessions[si].id.clone();
        let status_file = hooks::status_dir().join(&sid).join("status.json");
        let _ = std::fs::create_dir_all(status_file.parent().unwrap());
        let _ = std::fs::write(&status_file, r#"{"status":"running"}"#);
        if let Some(s) = self.projects[pi].sessions[si].pipeline_run.as_mut() {
            s.last_seen_running = false;
            s.prompt_pending_send = step.interactive;
            s.send_delay_ticks = if step.interactive { 1 } else { 0 };
        }
        self.projects[pi].sessions[si].status = SessionStatus::Running;
        self.projects[pi].sessions[si].status_changed_at = chrono::Utc::now();

        // Both modes: full claude TUI, prompt typed in after spawn. Difference is
        // only in auto-advance — non-interactive moves to next step on
        // Running→AwaitingInput; interactive waits for the user to click ▶ next.
        // (Earlier `claude -p` for non-interactive caused blank PTYs because print
        // mode exits immediately and we lose all output + status hooks.)
        if let Some(s) = self.projects[pi].sessions[si].pipeline_run.as_mut() {
            s.prompt_pending_send = true;
            s.send_delay_ticks = 1;
        }
        let _ = full_prompt; // built lazily inside PipelineSendPrompt
        let _ = claude_path; // command built via build_agent_command below
        // Per-step backend (claude / opencode / codex / mimo), else global.
        let backend = step.backend.as_deref().map(AgentBackend::from_str).unwrap_or(self.agent_backend);
        let session_name = self.projects[pi].sessions[si].name.clone();
        let (program, args) = build_agent_command(
            backend, true, false,
            &which_claude(), &which_opencode(), &which_codex(),
            &session_name, self.dangerously_skip_permissions,
        );
        self.spawn_terminal_with(pi, si, program, args);
        Task::perform(
            async { tokio::time::sleep(Duration::from_millis(1500)).await; },
            move |_| Message::PipelineSendPrompt(pi, si),
        )
    }

    /// Tear down terminal for (pi, si) and immediately spawn the next pipeline step
    /// (or finalize if this was the last step).
    fn advance_pipeline(&mut self, pi: usize, si: usize) -> Task<Message> {
        // Snapshot current_step + total before mutating.
        let (current, total) = match self.projects[pi].sessions[si].pipeline_run.as_ref() {
            Some(r) => (r.current_step, r.steps.len()),
            None => return Task::none(),
        };
        let next = current + 1;
        if next >= total {
            // Last step finished — leave terminal alive, clear pipeline_run.
            app_log!("pipeline finished pi={} si={}", pi, si);
            self.projects[pi].sessions[si].pipeline_run = None;
            self.save_state();
            return Task::none();
        }

        // Kill current terminal then spawn next step after a brief delay so the
        // PTY shuts down cleanly before we replace it.
        if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
            ti.terminal.handle(iced_term::Command::ProxyToBackend(
                iced_term::backend::Command::Write(b"\x03\x03".to_vec()),
            ));
            ti.terminal.handle(iced_term::Command::ProxyToBackend(
                iced_term::backend::Command::Write(b"exit\n".to_vec()),
            ));
        }
        self.terminals.retain(|(k, _)| *k != (pi, si));
        if let Some(r) = self.projects[pi].sessions[si].pipeline_run.as_mut() {
            r.current_step = next;
        }
        Task::perform(
            async { tokio::time::sleep(Duration::from_millis(500)).await; },
            move |_| Message::PipelineSpawnNext(pi, si),
        )
    }

    fn save_state(&self) {
        persistence::save(&persistence::AppState {
            projects: self.projects.clone(),
            theme: format!("{:?}", self.current_theme),
            sidebar_width: self.sidebar_width,
            groq_api_key: self.groq_api_key.clone(),
            dangerously_skip_permissions: self.dangerously_skip_permissions,
            quick_prompts: self.quick_prompts.clone(),
            date_prefix_enabled: self.date_prefix_enabled,
            notion_token: self.notion_token.clone(),
            notion_database_id: self.notion_database_id.clone(),
            notion_group_by_prop: self.notion_group_by_prop.clone(),
            agent_backend: self.agent_backend.as_str().to_string(),
            task_links: self.task_links.clone(),
            pipelines: self.pipelines.clone(),
        });
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let _guard = logging::perf(message_label(&message));
        match message {
            Message::OpenProject => {
                Task::perform(async {
                    rfd::AsyncFileDialog::new().set_title("Open existing project folder")
                        .pick_folder().await.map(|h| h.path().to_path_buf())
                }, Message::ProjectPicked)
            }
            Message::NewProject => {
                Task::perform(async {
                    rfd::AsyncFileDialog::new().set_title("Select PARENT folder for new project")
                        .pick_folder().await.map(|h| h.path().to_path_buf())
                }, Message::NewProjectFolderPicked)
            }
            Message::ProjectPicked(path) => {
                app_log!("ProjectPicked: {:?}", path);
                if let Some(p) = path {
                    if p.is_dir() {
                        let name = p.file_name().map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.display().to_string());
                        let mut group = ProjectGroup::new(name.clone(), p.clone());
                        if let Some(info) = git_info::get_git_info_with_pr(&p) {
                            group.branch = info.branch.clone();
                            group.dirty_files = info.dirty_files;
                            group.sub_repos = to_sub_repo_views(&info);
                        }
                        self.projects.push(group);
                        let pi = self.projects.len() - 1;
                        self.active_project = Some(pi);
                        self.file_entries = explorer::read_directory(&p, 0);
                        // Auto-open session name dialog with folder name as default
                        self.session_name_input = name;
                        self.show_session_dialog = Some(pi);
                        self.launch_agent = true;
                        self.save_state();
                    }
                }
                iced::widget::operation::focus_next()
            }
            Message::NewProjectFolderPicked(path) => {
                if let Some(parent) = path {
                    self.new_project_parent = Some(parent);
                    self.session_name_input = String::new();
                    self.show_session_dialog = Some(usize::MAX);
                    self.launch_agent = true;
                }
                iced::widget::operation::focus_next()
            }
            Message::ToggleLaunchClaude => { self.launch_agent = !self.launch_agent; Task::none() }
            Message::AddSession(pi) => {
                self.show_session_dialog = Some(pi);
                iced::widget::operation::focus_next()
            }
            Message::NewSessionInCurrentFolder => {
                // Open the session-name dialog for the currently active project,
                // so a new session is created in the same folder.
                match self.active_project {
                    Some(pi) if pi < self.projects.len() => {
                        self.session_name_input.clear();
                        self.show_session_dialog = Some(pi);
                        iced::widget::operation::focus_next()
                    }
                    _ => Task::none(),
                }
            }
            Message::SessionNameChanged(n) => { self.session_name_input = n; Task::none() }
            Message::CancelSessionDialog => {
                // If the dialog was opened by ProjectPicked (📁 Open folder) and
                // the user is now bailing out before naming a session, the just-
                // pushed ProjectGroup is invisible garbage (sidebar hides
                // sessionless projects). Roll it back so state stays clean.
                // For the "+ add session" flow on an existing project, the project
                // has >=1 session by definition (it's visible), so the check below
                // is a no-op there. The usize::MAX flow ("+" new project) doesn't
                // push until SessionNameSubmit, so nothing to undo either.
                let to_remove = self.show_session_dialog
                    .filter(|&pi| pi != usize::MAX
                        && pi < self.projects.len()
                        && self.projects[pi].sessions.is_empty());
                self.session_name_input.clear();
                self.show_session_dialog = None;
                self.new_project_parent = None;
                if let Some(pi) = to_remove {
                    return self.update(Message::RemoveProject(pi));
                }
                Task::none()
            }
            Message::SessionNameSubmit(pi, name) => {
                app_log!("SessionNameSubmit: pi={} name={}", pi, name);
                if !name.is_empty() {
                    if pi == usize::MAX {
                        // "+" flow: create new folder in parent
                        if let Some(parent) = self.new_project_parent.take() {
                            let folder_name = if self.date_prefix_enabled {
                                let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                                format!("{}-{}", date, name)
                            } else {
                                name.clone()
                            };
                            let new_path = parent.join(&folder_name);
                            app_log!("Creating new project folder: {:?}", new_path);
                            if let Err(e) = std::fs::create_dir_all(&new_path) {
                                app_log!("Failed to create folder: {}", e);
                            } else {
                                let mut group = ProjectGroup::new(folder_name.clone(), new_path.clone());
                                if let Some(info) = git_info::get_git_info_with_pr(&new_path) {
                                    group.branch = info.branch.clone(); group.dirty_files = info.dirty_files;
                                    group.sub_repos = to_sub_repo_views(&info);
                                }
                                self.projects.push(group);
                                let new_pi = self.projects.len() - 1;
                                let mut session = Session::new(name);
                                session.color = self.new_session_color;
                                self.projects[new_pi].sessions.push(session);
                                self.active_project = Some(new_pi);
                                self.active_session = Some((new_pi, 0));
                                self.file_entries = explorer::read_directory(&new_path, 0);
                                self.spawn_session_terminal(new_pi, 0, false);
                                self.save_state();
                            }
                        }
                    } else if pi < self.projects.len() {
                        // Normal "add session" flow
                        let mut session = Session::new(name);
                        session.color = self.new_session_color;
                        self.projects[pi].sessions.push(session);
                        let si = self.projects[pi].sessions.len() - 1;
                        self.active_session = Some((pi, si));
                        self.spawn_session_terminal(pi, si, false);
                        self.save_state();
                    }
                }
                self.session_name_input.clear();
                self.show_session_dialog = None;
                self.new_project_parent = None;
                Task::none()
            }
            Message::CardHover(v) => { self.hovered_card = v; Task::none() }
            Message::SelectSession(pi, si) => {
                app_log!("SelectSession: pi={} si={}", pi, si);
                if pi >= self.projects.len() || si >= self.projects[pi].sessions.len() {
                    app_log!("  SelectSession ignored: stale indices (projects={}, sessions={})",
                        self.projects.len(),
                        self.projects.get(pi).map(|p| p.sessions.len()).unwrap_or(0));
                    return Task::none();
                }
                self.active_session = Some((pi, si));
                self.active_project = Some(pi);
                self.file_entries = explorer::read_directory(&self.projects[pi].path, 0);
                self.show_deployment_dropdown = false;
                // Spawn terminal if it doesn't exist (e.g. after restart). If the
                // session has a persisted pipeline_run, resume the current step
                // rather than doing a plain `claude --continue` so the prompt
                // gets re-injected and progress tracking stays consistent.
                let has_term = self.terminals.iter().any(|(k, _)| *k == (pi, si));
                let has_pipeline = self.projects[pi].sessions[si].pipeline_run.is_some();
                let mut tasks: Vec<Task<Message>> = Vec::new();
                if !has_term {
                    if has_pipeline {
                        tasks.push(self.spawn_pipeline_step(pi, si));
                    } else {
                        self.spawn_session_terminal(pi, si, true);
                    }
                } else if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                    // Foregrounded — refresh its rendered grid (it was handled
                    // quietly while in the background to save CPU/GPU).
                    ti.terminal.sync();
                }
                tasks.push(self.update(Message::FetchGitInfo(pi)));
                tasks.push(self.update(Message::FetchDeployments(pi)));
                return Task::batch(tasks);
            }
            Message::MakeIdle(pi, si) => {
                if pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.projects[pi].sessions[si].status = SessionStatus::Idle;
                    self.projects[pi].sessions[si].status_changed_at = chrono::Utc::now();
                    // Write idle to status.json so Tick doesn't overwrite
                    let sid = &self.projects[pi].sessions[si].id;
                    let status_file = hooks::status_dir().join(sid).join("status.json");
                    let _ = std::fs::write(&status_file, r#"{"status":"idle"}"#);
                }
                Task::none()
            }
            Message::KillSession(pi, si) => {
                app_log!("KillSession: pi={} si={}", pi, si);
                // Send Ctrl+C and exit to gracefully stop claude
                if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(b"\x03\x03".to_vec())
                    ));
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(b"exit\n".to_vec())
                    ));
                }
                // Wait 500ms then clean up
                Task::perform(
                    async { tokio::time::sleep(Duration::from_millis(500)).await; },
                    move |_| Message::TerminalExited(pi, si),
                )
            }
            Message::TerminalExited(pi, si) => {
                app_log!("TerminalExited: pi={} si={}", pi, si);
                self.terminals.retain(|(k, _)| *k != (pi, si));
                if pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.projects[pi].sessions.remove(si);
                }
                // Fix active_session: drop it if it pointed at the removed session,
                // shift index down if it pointed at a later session in the same project.
                match self.active_session {
                    Some((ap, asi)) if ap == pi && asi == si => self.active_session = None,
                    Some((ap, asi)) if ap == pi && asi > si => self.active_session = Some((ap, asi - 1)),
                    _ => {}
                }
                for (key, _) in &mut self.terminals {
                    if key.0 == pi && key.1 > si { key.1 -= 1; }
                }
                self.save_state();
                Task::none()
            }
            Message::RemoveProject(pi) => {
                app_log!("RemoveProject: pi={}", pi);
                if pi >= self.projects.len() {
                    return Task::none(); // Already removed, ignore duplicate
                }
                // Kill all terminals for this project
                let session_count = self.projects[pi].sessions.len();
                for si in (0..session_count).rev() {
                    self.terminals.retain(|(k, _)| *k != (pi, si));
                }
                // Fix terminal keys for projects after this one
                for (key, _) in &mut self.terminals {
                    if key.0 > pi { key.0 -= 1; }
                }
                // Remove project
                if pi < self.projects.len() {
                    self.projects.remove(pi);
                }
                // Fix active session
                if let Some((ap, _)) = self.active_session {
                    if ap == pi { self.active_session = None; }
                    else if ap > pi { self.active_session = Some((ap - 1, self.active_session.unwrap().1)); }
                }
                if self.active_project == Some(pi) { self.active_project = None; }
                else if self.active_project.map(|a| a > pi).unwrap_or(false) {
                    self.active_project = Some(self.active_project.unwrap() - 1);
                }
                self.save_state();
                Task::none()
            }
            Message::DeleteProjectDir(pi) => {
                if pi < self.projects.len() {
                    let path = &self.projects[pi].path;
                    // Collect uncommitted files from all sub-repos
                    let mut uncommitted = Vec::new();
                    let git_dirs = git_info::find_git_repos_pub(path, 3);
                    for gd in &git_dirs {
                        if let Ok(out) = std::process::Command::new("git")
                            .args(["status", "--porcelain"])
                            .current_dir(gd)
                            .output()
                        {
                            let prefix = gd.strip_prefix(path).unwrap_or(gd);
                            for line in String::from_utf8_lossy(&out.stdout).lines() {
                                if !line.is_empty() {
                                    let file = line.get(3..).unwrap_or(line);
                                    uncommitted.push(format!("{}/{}", prefix.display(), file));
                                }
                            }
                        }
                    }
                    self.confirm_delete = Some((pi, uncommitted));
                }
                Task::none()
            }
            Message::ConfirmDeleteDir(pi) => {
                if pi < self.projects.len() {
                    // Send Ctrl+C + exit to all terminals in this project
                    let session_count = self.projects[pi].sessions.len();
                    for si in 0..session_count {
                        if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                            ti.terminal.handle(iced_term::Command::ProxyToBackend(
                                iced_term::backend::Command::Write(b"\x03\x03".to_vec())
                            ));
                            ti.terminal.handle(iced_term::Command::ProxyToBackend(
                                iced_term::backend::Command::Write(b"exit\n".to_vec())
                            ));
                        }
                    }
                    self.confirm_delete = None;
                    // Wait 800ms for terminals to die, then delete
                    return Task::perform(
                        async { tokio::time::sleep(Duration::from_millis(800)).await; },
                        move |_| Message::ProjectTerminalsExited(pi),
                    );
                }
                self.confirm_delete = None;
                Task::none()
            }
            Message::ProjectTerminalsExited(pi) => {
                if pi < self.projects.len() {
                    let path = self.projects[pi].path.clone();
                    app_log!("TRASHING directory: {:?}", path);
                    // Move to Trash on a blocking worker thread — never block the UI thread.
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                trash::delete(&path).map_err(|e| e.to_string())
                            })
                            .await
                            .map_err(|e| e.to_string())
                            .and_then(|r| r)
                        },
                        move |res| Message::ProjectDirTrashed(pi, res),
                    );
                }
                Task::none()
            }
            Message::ProjectDirTrashed(pi, res) => {
                match &res {
                    Ok(_) => app_log!("Trashed project dir: pi={}", pi),
                    Err(e) => app_log!("Failed to trash: {}", e),
                }
                // Always remove project from the list even if trash failed — the user
                // asked for it, and they can see the error via logs.
                return self.update(Message::RemoveProject(pi));
            }
            Message::StartRenameSession(pi, si) => {
                if pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.rename_input = self.projects[pi].sessions[si].name.clone();
                    self.renaming_session = Some((pi, si));
                }
                iced::widget::operation::focus_next()
            }
            Message::RenameSessionInput(s) => { self.rename_input = s; Task::none() }
            Message::SetSessionColor(pi, si, color) => {
                if pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.projects[pi].sessions[si].color = color;
                    self.save_state();
                }
                Task::none()
            }
            Message::SetNewSessionColor(color) => {
                self.new_session_color = color;
                Task::none()
            }
            Message::RenameSessionSubmit => {
                if let Some((pi, si)) = self.renaming_session {
                    if pi < self.projects.len() && si < self.projects[pi].sessions.len() && !self.rename_input.is_empty() {
                        self.projects[pi].sessions[si].name = self.rename_input.clone();
                        self.save_state();
                    }
                }
                self.renaming_session = None;
                self.rename_input.clear();
                Task::none()
            }
            Message::RefreshProject(pi) => {
                let git_task = self.update(Message::FetchGitInfo(pi));
                let dep_task = self.update(Message::FetchDeployments(pi));
                return Task::batch([git_task, dep_task]);
            }
            Message::FetchGitInfo(pi) => {
                app_log!("FetchGitInfo: pi={}", pi);
                if pi >= self.projects.len() { return Task::none(); }
                self.refresh_pending_git.insert(pi);
                let path = self.projects[pi].path.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || git_info::get_git_info_with_pr(&path))
                            .await.ok().flatten()
                    },
                    move |info| Message::GitInfoFetched(pi, info),
                )
            }
            Message::GitInfoFetched(pi, info) => {
                app_log!("GitInfoFetched: pi={} has_info={}", pi, info.is_some());
                self.refresh_pending_git.remove(&pi);
                if let Some(info) = info {
                    if pi < self.projects.len() {
                        self.projects[pi].branch = info.branch.clone();
                        self.projects[pi].dirty_files = info.dirty_files;
                        let old_deps: std::collections::HashMap<String, Vec<session::DeploymentInfo>> = self.projects[pi].sub_repos.iter()
                            .map(|sr| (sr.name.clone(), sr.deployments.clone())).collect();
                        self.projects[pi].sub_repos = to_sub_repo_views(&info);
                        for sr in &mut self.projects[pi].sub_repos {
                            if let Some(deps) = old_deps.get(&sr.name) {
                                sr.deployments = deps.clone();
                            }
                        }
                        // dev_servers already populated from git_info (includes TCP check)
                    }
                }
                Task::none()
            }
            Message::FetchDeployments(pi) => {
                if pi >= self.projects.len() { return Task::none(); }
                self.refresh_pending_deps.insert(pi);
                let sub_repos: Vec<(String, PathBuf)> = {
                    let git_dirs = git_info::find_git_repos_pub(&self.projects[pi].path, 3);
                    git_dirs.iter().map(|gd| {
                        let name = gd.strip_prefix(&self.projects[pi].path)
                            .map(|p| { let s = p.to_string_lossy().to_string(); if s.is_empty() { ".".to_string() } else { s } })
                            .unwrap_or_else(|_| ".".to_string());
                        (name, gd.clone())
                    }).collect()
                };
                app_log!("FetchDeployments: pi={} repos={}", pi, sub_repos.len());
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let mut results = Vec::new();
                            for (name, path) in sub_repos {
                                let deps = git_info::get_deployments(&path);
                                if !deps.is_empty() {
                                    results.push((name, deps));
                                }
                            }
                            (pi, results)
                        }).await.unwrap_or((pi, Vec::new()))
                    },
                    |(pi, deps)| Message::DeploymentsFetched(pi, deps),
                )
            }
            Message::DeploymentsFetched(pi, deps) => {
                app_log!("DeploymentsFetched: pi={} total={}", pi, deps.iter().map(|(_, d)| d.len()).sum::<usize>());
                self.refresh_pending_deps.remove(&pi);
                if pi < self.projects.len() {
                    for sr in &mut self.projects[pi].sub_repos {
                        sr.deployments.clear();
                        for (name, repo_deps) in &deps {
                            if *name == sr.name {
                                sr.deployments = repo_deps.iter().map(|(env, state, url)| {
                                    session::DeploymentInfo {
                                        env: env.clone(), state: state.clone(), url: url.clone(),
                                    }
                                }).collect();
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ToggleDeploymentDropdown => {
                self.show_deployment_dropdown = !self.show_deployment_dropdown;
                Task::none()
            }
            Message::OpenUrl(url) => {
                let _ = std::process::Command::new("open").arg(&url).spawn();
                Task::none()
            }
            Message::OpenFile(path) => {
                app_log!("OpenFile: {:?}", path);
                let _ = std::process::Command::new("open").arg(&path).spawn();
                Task::none()
            }
            Message::VoiceToggle => {
                if self.voice_recorder.is_recording {
                    // Stop recording and transcribe
                    let wav = self.voice_recorder.stop();
                    app_log!("Voice: stopped, {} bytes WAV", wav.len());
                    if self.groq_api_key.is_empty() {
                        app_log!("Voice: no Groq API key set!");
                        return Task::none();
                    }
                    self.voice_transcribing = true;
                    let key = self.groq_api_key.clone();
                    Task::perform(
                        async move { voice::transcribe_groq(wav, &key).await },
                        Message::VoiceResult,
                    )
                } else {
                    // Start recording
                    match self.voice_recorder.start() {
                        Ok(()) => app_log!("Voice: recording started"),
                        Err(e) => app_log!("Voice: failed to start: {}", e),
                    }
                    Task::none()
                }
            }
            Message::VoiceResult(result) => {
                self.voice_transcribing = false;
                match result {
                    Ok(text) => {
                        app_log!("Voice: transcribed: {}", text);
                        // Write text to active terminal
                        if let Some((pi, si)) = self.active_session {
                            if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                                ti.terminal.handle(iced_term::Command::ProxyToBackend(
                                    iced_term::backend::Command::Write(text.into_bytes())
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        app_log!("Voice: transcription error: {}", e);
                    }
                }
                Task::none()
            }
            Message::ToggleDangerouslySkipPermissions => {
                self.dangerously_skip_permissions = !self.dangerously_skip_permissions;
                self.save_state();
                Task::none()
            }
            Message::ToggleDatePrefix => {
                self.date_prefix_enabled = !self.date_prefix_enabled;
                self.save_state();
                Task::none()
            }
            Message::SendQuickPrompt(prompt) => {
                // Type prompt into active terminal
                if let Some((pi, si)) = self.active_session {
                    if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                        let bytes = (prompt + "\n").into_bytes();
                        ti.terminal.handle(iced_term::Command::ProxyToBackend(
                            iced_term::backend::Command::Write(bytes)
                        ));
                    }
                }
                Task::none()
            }
            Message::AddQuickPrompt => {
                if !self.quick_prompt_input.is_empty() {
                    self.quick_prompts.push(self.quick_prompt_input.clone());
                    self.quick_prompt_input.clear();
                    self.save_state();
                }
                Task::none()
            }
            Message::RemoveQuickPrompt(idx) => {
                if idx < self.quick_prompts.len() {
                    self.quick_prompts.remove(idx);
                    self.save_state();
                }
                Task::none()
            }
            Message::QuickPromptInput(s) => { self.quick_prompt_input = s; Task::none() }
            Message::UpdateCheckResult(version) => {
                if let Some(v) = version {
                    app_log!("Update available: {}", v);
                    self.update_available = Some(v);
                }
                Task::none()
            }
            Message::DismissUpdate => {
                self.update_available = None;
                Task::none()
            }
            Message::GroqKeyChanged(key) => {
                self.groq_api_key = key;
                self.save_state();
                Task::none()
            }
            Message::CancelDelete => { self.confirm_delete = None; Task::none() }
            Message::ToggleProjectExpand(_) => Task::none(),
            Message::ToggleFileExpand(idx) => {
                if idx < self.file_entries.len() && matches!(self.file_entries[idx].kind, explorer::FileEntryKind::Directory) {
                    let exp = self.file_entries[idx].expanded;
                    let path = self.file_entries[idx].path.clone();
                    let depth = self.file_entries[idx].depth;
                    self.file_entries[idx].expanded = !exp;
                    if !exp {
                        let children = explorer::read_directory(&path, depth + 1);
                        for (i, ch) in children.into_iter().enumerate() { self.file_entries.insert(idx + 1 + i, ch); }
                    } else {
                        let mut n = 0;
                        for i in (idx+1)..self.file_entries.len() { if self.file_entries[i].depth > depth { n += 1; } else { break; } }
                        self.file_entries.drain((idx+1)..(idx+1+n));
                    }
                }
                Task::none()
            }
            Message::RefreshExplorer => {
                if let Some(pi) = self.active_project { self.file_entries = explorer::read_directory(&self.projects[pi].path, 0); }
                Task::none()
            }
            Message::Blink => {
                self.blink_on = !self.blink_on;
                self.tick_count = self.tick_count.wrapping_add(1);
                Task::none()
            }
            Message::FrameProbe => {
                // Measure delta between probe ticks. Probe fires every 100ms;
                // a delta noticeably > 100ms means the UI thread was blocked.
                let now = std::time::Instant::now();
                if let Some(prev) = self.last_frame_probe {
                    let delta_ms = now.duration_since(prev).as_millis() as u64;
                    // Record total delta (raw) so flush shows max — easy to spot freezes.
                    logging::metrics::record("FrameProbe.delta_ms", delta_ms * 1000);
                    // Anything past 250ms is a visible stutter — log it eagerly.
                    if delta_ms > 250 {
                        app_log!("[FREEZE] UI thread blocked for {}ms", delta_ms);
                    }
                }
                self.last_frame_probe = Some(now);
                Task::none()
            }
            Message::KeyboardEvent(event) => {
                if let iced::keyboard::Event::KeyPressed { key, modifiers, physical_key, .. } = event {
                    if modifiers.command() {
                        // Use physical key for non-latin layouts
                        let latin = key.to_latin(physical_key);
                        match latin {
                            Some('o') => return self.update(Message::OpenProject),
                            Some('n') => return self.update(Message::NewProject),
                            Some('t') => return self.update(Message::NewSessionInCurrentFolder),
                            _ => {}
                        }
                    }
                }
                Task::none()
            }
            Message::Tick => {
                // Only check session statuses via hooks (lightweight, no network)
                let mut advance: Vec<(usize, usize)> = Vec::new();
                for (pi, p) in self.projects.iter_mut().enumerate() {
                    for (si, s) in p.sessions.iter_mut().enumerate() {
                        if let Some(pay) = hooks::read_status(&s.id) {
                            if let Some(st) = pay.to_session_status() {
                                if st != s.status {
                                    s.status_changed_at = chrono::Utc::now();
                                    s.status = st;
                                }
                            }
                            s.last_was_question = pay.last_was_question.unwrap_or(false);
                        }
                        // Pipeline driver: auto-advance only for non-interactive steps.
                        // Interactive steps stop the train — the user clicks ▶ Next manually.
                        // Also block advance when claude's last turn was a question — it's
                        // asking the user something, not finishing work.
                        if let Some(run) = s.pipeline_run.as_mut() {
                            if matches!(s.status, SessionStatus::Running) {
                                run.last_seen_running = true;
                            }
                            if run.prompt_pending_send && run.send_delay_ticks > 0 {
                                run.send_delay_ticks = run.send_delay_ticks.saturating_sub(1);
                            }
                            let current_interactive = run.steps.get(run.current_step)
                                .map(|st| st.interactive).unwrap_or(false);
                            let done = !current_interactive
                                && run.last_seen_running
                                && !s.last_was_question
                                && matches!(s.status, SessionStatus::AwaitingInput | SessionStatus::Done);
                            if done {
                                advance.push((pi, si));
                            }
                        }
                    }
                }
                let mut tasks: Vec<Task<Message>> = Vec::new();
                for (pi, si) in advance {
                    tasks.push(self.advance_pipeline(pi, si));
                }
                // Dump accumulated perf metrics so we can see what dominates CPU
                logging::metrics::flush(1);
                if tasks.is_empty() { Task::none() } else { Task::batch(tasks) }
            }
            Message::AddPipeline => {
                let n = self.pipelines.len() + 1;
                self.pipelines.push(PipelineDef::new(format!("Pipeline {}", n)));
                self.editing_pipeline = Some(self.pipelines.len() - 1);
                self.save_state();
                Task::none()
            }
            Message::RemovePipeline(idx) => {
                if idx < self.pipelines.len() {
                    self.pipelines.remove(idx);
                    if self.editing_pipeline == Some(idx) { self.editing_pipeline = None; }
                    else if let Some(e) = self.editing_pipeline {
                        if e > idx { self.editing_pipeline = Some(e - 1); }
                    }
                    self.save_state();
                }
                Task::none()
            }
            Message::PipelineNameChanged(idx, name) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    p.name = name;
                    self.save_state();
                }
                Task::none()
            }
            Message::PipelinePrependChanged(idx, val) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    p.prepend_template = val;
                    self.save_state();
                }
                Task::none()
            }
            Message::AddPipelineStep(idx) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    p.steps.push(PipelineStep::default());
                    self.save_state();
                }
                Task::none()
            }
            Message::RemovePipelineStep(idx, step_idx) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    if step_idx < p.steps.len() {
                        p.steps.remove(step_idx);
                        self.save_state();
                    }
                }
                Task::none()
            }
            Message::MovePipelineStep(idx, step_idx, delta) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    let new_idx = step_idx as isize + delta;
                    if step_idx < p.steps.len() && new_idx >= 0 && (new_idx as usize) < p.steps.len() {
                        p.steps.swap(step_idx, new_idx as usize);
                        self.save_state();
                    }
                }
                Task::none()
            }
            Message::PipelineStepPromptChanged(idx, step_idx, prompt) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    if let Some(s) = p.steps.get_mut(step_idx) {
                        s.prompt = prompt;
                        self.save_state();
                    }
                }
                Task::none()
            }
            Message::TogglePipelineStepInteractive(idx, step_idx) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    if let Some(s) = p.steps.get_mut(step_idx) {
                        s.interactive = !s.interactive;
                        self.save_state();
                    }
                }
                Task::none()
            }
            Message::CyclePipelineStepBackend(idx, step_idx) => {
                if let Some(p) = self.pipelines.get_mut(idx) {
                    if let Some(s) = p.steps.get_mut(step_idx) {
                        // None(global) → Claude → OpenCode → Codex → MiMo → None
                        let order: [Option<&str>; 5] =
                            [None, Some("ClaudeCode"), Some("OpenCode"), Some("Codex"), Some("XiaomiMimo")];
                        let cur = order.iter().position(|o| o.map(str::to_string) == s.backend).unwrap_or(0);
                        s.backend = order[(cur + 1) % order.len()].map(str::to_string);
                        self.save_state();
                    }
                }
                Task::none()
            }
            Message::ToggleEditPipeline(idx) => {
                self.editing_pipeline = if self.editing_pipeline == Some(idx) { None } else { Some(idx) };
                Task::none()
            }
            Message::OpenRunPipelineModal(pi, si) => {
                self.show_pipeline_modal = Some((pi, si));
                self.pipeline_modal_selected = if self.pipelines.is_empty() { None } else { Some(0) };
                self.pipeline_modal_requirements = text_editor::Content::new();
                Task::none()
            }
            Message::ClosePipelineModal => {
                self.show_pipeline_modal = None;
                self.pipeline_modal_selected = None;
                self.pipeline_modal_requirements = text_editor::Content::new();
                Task::none()
            }
            Message::SelectPipelineForRun(idx) => {
                self.pipeline_modal_selected = Some(idx);
                Task::none()
            }
            Message::PipelineRequirementsAction(action) => {
                self.pipeline_modal_requirements.perform(action);
                Task::none()
            }
            Message::StartPipeline => {
                let Some((pi, si)) = self.show_pipeline_modal else { return Task::none(); };
                let Some(p_idx) = self.pipeline_modal_selected else { return Task::none(); };
                if pi >= self.projects.len() || si >= self.projects[pi].sessions.len() { return Task::none(); }
                let Some(pipeline) = self.pipelines.get(p_idx).cloned() else { return Task::none(); };
                if pipeline.steps.is_empty() { return Task::none(); }

                // Save requirements file at project root.
                let req_path = self.projects[pi].path.join(".orchpipeline");
                let req_text = self.pipeline_modal_requirements.text();
                if let Err(e) = std::fs::write(&req_path, &req_text) {
                    app_log!("StartPipeline: failed to write {}: {}", req_path.display(), e);
                    return Task::none();
                }

                // Kill any existing terminal for this session before driving the pipeline.
                if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(b"\x03\x03".to_vec()),
                    ));
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(b"exit\n".to_vec()),
                    ));
                }
                self.terminals.retain(|(k, _)| *k != (pi, si));

                self.projects[pi].sessions[si].pipeline_run = Some(PipelineRun {
                    pipeline_name: pipeline.name.clone(),
                    steps: pipeline.steps.clone(),
                    current_step: 0,
                    requirements_path: req_path,
                    prepend_template: pipeline.prepend_template.clone(),
                    last_seen_running: false,
                    prompt_pending_send: false,
                    send_delay_ticks: 0,
                });
                self.active_session = Some((pi, si));
                self.active_project = Some(pi);
                self.show_pipeline_modal = None;
                self.pipeline_modal_selected = None;
                self.pipeline_modal_requirements = text_editor::Content::new();
                self.save_state();

                // Defer spawn slightly so the killed terminal is fully gone.
                Task::perform(
                    async { tokio::time::sleep(Duration::from_millis(400)).await; },
                    move |_| Message::PipelineSpawnNext(pi, si),
                )
            }
            Message::PipelineSpawnNext(pi, si) => {
                if pi >= self.projects.len() || si >= self.projects[pi].sessions.len() { return Task::none(); }
                self.spawn_pipeline_step(pi, si)
            }
            Message::PipelineSendPrompt(pi, si) => {
                if pi >= self.projects.len() || si >= self.projects[pi].sessions.len() { return Task::none(); }
                let Some(run) = self.projects[pi].sessions[si].pipeline_run.as_ref() else {
                    return Task::none();
                };
                let Some(step) = run.steps.get(run.current_step) else { return Task::none(); };
                let prompt = Self::build_step_prompt(
                    &step.prompt,
                    &run.requirements_path,
                    &run.prepend_template,
                    &self.projects[pi].path,
                    run.current_step,
                    run.steps.len(),
                );
                if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                    // Bracketed paste so claude TUI treats the whole multi-line
                    // prompt as one paste block (newlines stay literal). Then a
                    // separate Enter submits.
                    let mut bytes = Vec::with_capacity(prompt.len() + 16);
                    bytes.extend_from_slice(b"\x1b[200~");
                    bytes.extend_from_slice(prompt.as_bytes());
                    bytes.extend_from_slice(b"\x1b[201~");
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(bytes),
                    ));
                    // Schedule the submit (Enter) shortly after so claude has
                    // a chance to fully ingest the paste block before submit.
                    let pi_c = pi; let si_c = si;
                    return Task::perform(
                        async { tokio::time::sleep(Duration::from_millis(150)).await; },
                        move |_| Message::PipelineSubmitPrompt(pi_c, si_c),
                    );
                }
                if let Some(r) = self.projects[pi].sessions[si].pipeline_run.as_mut() {
                    r.prompt_pending_send = false;
                }
                Task::none()
            }
            Message::PipelineSubmitPrompt(pi, si) => {
                if pi >= self.projects.len() || si >= self.projects[pi].sessions.len() { return Task::none(); }
                if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(b"\r".to_vec()),
                    ));
                }
                Task::none()
            }
            Message::PipelineNextStep(pi, si) => {
                if pi >= self.projects.len() || si >= self.projects[pi].sessions.len() { return Task::none(); }
                self.advance_pipeline(pi, si)
            }
            Message::PipelineJumpToStep(pi, si, step_idx) => {
                if pi >= self.projects.len() || si >= self.projects[pi].sessions.len() { return Task::none(); }
                let total = match self.projects[pi].sessions[si].pipeline_run.as_ref() {
                    Some(r) => r.steps.len(),
                    None => return Task::none(),
                };
                if step_idx >= total { return Task::none(); }

                // Tear down current terminal, then respawn at requested step.
                if let Some((_, ti)) = self.terminals.iter_mut().find(|(k, _)| *k == (pi, si)) {
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(b"\x03\x03".to_vec()),
                    ));
                    ti.terminal.handle(iced_term::Command::ProxyToBackend(
                        iced_term::backend::Command::Write(b"exit\n".to_vec()),
                    ));
                }
                self.terminals.retain(|(k, _)| *k != (pi, si));
                if let Some(r) = self.projects[pi].sessions[si].pipeline_run.as_mut() {
                    r.current_step = step_idx;
                }
                Task::perform(
                    async { tokio::time::sleep(Duration::from_millis(500)).await; },
                    move |_| Message::PipelineSpawnNext(pi, si),
                )
            }
            Message::PipelineStop(pi, si) => {
                if pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.projects[pi].sessions[si].pipeline_run = None;
                    self.save_state();
                }
                Task::none()
            }
            Message::RefreshAll => {
                app_log!("RefreshAll");
                let _perf = logging::perf("RefreshAll");
                for p in &mut self.projects {
                    if let Some(info) = git_info::get_git_info_with_pr(&p.path) {
                        p.branch = info.branch.clone(); p.dirty_files = info.dirty_files;
                        p.sub_repos = to_sub_repo_views(&info);
                    }
                }
                Task::none()
            }
            Message::ResizeSidebar(delta) => {
                self.sidebar_width = (self.sidebar_width + delta).clamp(180.0, 500.0);
                self.save_state();
                Task::none()
            }
            Message::ToggleSettings => { self.show_settings = !self.show_settings; Task::none() }
            Message::SetTheme(t) => { self.current_theme = t; self.save_state(); Task::none() }
            Message::TermEvent(event) => {
                match event {
                    iced_term::Event::BackendCall(id, cmd) => {
                        // Only the focused, on-screen terminal does the heavy grid
                        // sync + GPU redraw. Every other terminal is still drained
                        // (the subscription consumes the channel) and kept PTY-
                        // responsive via handle_quiet, but skips sync/redraw — this
                        // is the main battery/heat win when background sessions run
                        // animated TUIs that flood output.
                        let active_key = self.active_session;
                        let on_ide = self.screen == Screen::Ide;
                        if let Some((key, ti)) = self.terminals.iter_mut().find(|(_, t)| t.id == id) {
                            if on_ide && Some(*key) == active_key {
                                ti.terminal.handle(iced_term::Command::ProxyToBackend(cmd));
                            } else {
                                ti.terminal.handle_quiet(iced_term::Command::ProxyToBackend(cmd));
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::SetAgentBackend(b) => { self.agent_backend = b; self.save_state(); Task::none() }
            Message::SetScreen(s) => { self.screen = s; Task::none() }
            Message::NotionTokenChanged(t) => { self.notion_token = t; self.save_state(); Task::none() }
            Message::NotionConnect => {
                let tok = self.notion_token.clone();
                if tok.is_empty() { return Task::none(); }
                self.notion_loading = true; self.notion_error = None;
                Task::perform(notion::list_databases(tok), Message::NotionDatabasesFetched)
            }
            Message::NotionDatabasesFetched(res) => {
                self.notion_loading = false;
                match res {
                    Ok(dbs) => { self.notion_databases = dbs; self.notion_error = None; }
                    Err(e) => self.notion_error = Some(e),
                }
                Task::none()
            }
            Message::NotionSelectDatabase(id) => {
                self.notion_database_id = Some(id.clone());
                self.notion_group_by_prop = None;
                self.save_state();
                let tok = self.notion_token.clone();
                self.notion_loading = true;
                Task::perform(notion::fetch_schema(tok, id), Message::NotionSchemaFetched)
            }
            Message::NotionSchemaFetched(res) => {
                match res {
                    Ok(db) => {
                        if self.notion_group_by_prop.is_none() {
                            self.notion_group_by_prop = db.props.iter()
                                .find(|p| matches!(p.kind, notion::PropKind::Status(_)))
                                .or_else(|| db.props.iter().find(|p| matches!(p.kind, notion::PropKind::Select(_))))
                                .map(|p| p.id.clone());
                        }
                        let tok = self.notion_token.clone();
                        let id = db.id.clone();
                        let props = db.props.clone();
                        self.notion_schema = Some(db);
                        self.save_state();
                        return Task::perform(notion::query_tasks(tok, id, props), Message::NotionTasksFetched);
                    }
                    Err(e) => { self.notion_loading = false; self.notion_error = Some(e); }
                }
                Task::none()
            }
            Message::NotionTasksFetched(res) => {
                self.notion_loading = false;
                match res {
                    Ok(tasks) => { self.notion_tasks = tasks; self.notion_error = None; }
                    Err(e) => self.notion_error = Some(e),
                }
                Task::none()
            }
            Message::NotionRefresh => {
                if let Some(db) = self.notion_schema.clone() {
                    let tok = self.notion_token.clone();
                    self.notion_loading = true;
                    return Task::perform(notion::query_tasks(tok, db.id.clone(), db.props.clone()), Message::NotionTasksFetched);
                }
                Task::none()
            }
            Message::NotionSetGroupBy(pid) => { self.notion_group_by_prop = Some(pid); self.save_state(); Task::none() }
            Message::BoardAddTaskStart(col) => {
                self.adding_task_col = Some(col);
                self.new_task_title.clear();
                Task::none()
            }
            Message::BoardAddTaskInput(s) => { self.new_task_title = s; Task::none() }
            Message::BoardAddTaskCancel => { self.adding_task_col = None; self.new_task_title.clear(); Task::none() }
            Message::BoardAddTaskSubmit => {
                let title = self.new_task_title.trim().to_string();
                let (Some(col), Some(schema), Some(db_id)) =
                    (self.adding_task_col.clone(), self.notion_schema.clone(), self.notion_database_id.clone())
                    else { return Task::none(); };
                if title.is_empty() { return Task::none(); }
                let title_prop = match schema.props.iter().find(|p| matches!(p.kind, notion::PropKind::Title)) {
                    Some(p) => p.name.clone(),
                    None => { self.notion_error = Some("No title property in database".into()); return Task::none(); }
                };
                // group value for the target column (None for the "(none)" bucket)
                let gp_id = self.notion_group_by_prop.clone().unwrap_or_default();
                let group = schema.props.iter().find(|p| p.id == gp_id).and_then(|p| {
                    let kind = match p.kind {
                        notion::PropKind::Status(_) => "status",
                        notion::PropKind::Select(_) => "select",
                        _ => return None,
                    };
                    if col == "(none)" { None } else { Some((p.name.clone(), kind.to_string(), col.clone())) }
                });
                self.creating_task = true;
                self.notion_error = None;
                let tok = self.notion_token.clone();
                Task::perform(notion::create_task(tok, db_id, title_prop, title, group), Message::BoardTaskCreated)
            }
            Message::BoardTaskCreated(res) => {
                self.creating_task = false;
                match res {
                    Ok(()) => {
                        self.adding_task_col = None;
                        self.new_task_title.clear();
                        // refetch tasks to show the new card
                        if let Some(db) = self.notion_schema.clone() {
                            let tok = self.notion_token.clone();
                            self.notion_loading = true;
                            return Task::perform(notion::query_tasks(tok, db.id.clone(), db.props.clone()), Message::NotionTasksFetched);
                        }
                    }
                    Err(e) => self.notion_error = Some(e),
                }
                Task::none()
            }
            Message::OpenTaskBoard(id) => {
                self.screen = Screen::Board;
                return self.update(Message::NotionOpenTask(id));
            }
            Message::NotionOpenTask(id) => {
                self.open_task = Some(id.clone());
                self.new_session_backend = self.agent_backend; // default to global
                self.open_task_body = Vec::new();
                self.open_task_body_loading = true;
                self.open_task_comments = Vec::new();
                self.comments_loading = true;
                self.comment_input.clear();
                let tok = self.notion_token.clone();
                let id_b = id.clone();
                let id_c = id.clone();
                Task::batch([
                    Task::perform(notion::fetch_blocks(tok.clone(), id.clone()), move |res| Message::NotionBodyFetched(id_b.clone(), res)),
                    Task::perform(notion::fetch_comments(tok, id.clone()), move |res| Message::NotionCommentsFetched(id_c.clone(), res)),
                ])
            }
            Message::NotionBodyFetched(id, res) => {
                // ignore stale results for a task that's no longer open
                if self.open_task.as_deref() == Some(id.as_str()) {
                    self.open_task_body_loading = false;
                    match res {
                        Ok(blocks) => self.open_task_body = blocks,
                        Err(e) => { self.open_task_body = Vec::new(); self.notion_error = Some(e); }
                    }
                }
                Task::none()
            }
            Message::NotionCommentsFetched(id, res) => {
                if self.open_task.as_deref() == Some(id.as_str()) {
                    self.comments_loading = false;
                    match res {
                        Ok(cs) => self.open_task_comments = cs,
                        Err(e) => { self.open_task_comments = Vec::new(); self.notion_error = Some(e); }
                    }
                }
                Task::none()
            }
            Message::CommentInput(s) => { self.comment_input = s; Task::none() }
            Message::PostComment => {
                let text = self.comment_input.trim().to_string();
                if text.is_empty() { return Task::none(); }
                let Some(id) = self.open_task.clone() else { return Task::none(); };
                self.comment_posting = true;
                let tok = self.notion_token.clone();
                Task::perform(notion::create_comment(tok, id, text), Message::CommentPosted)
            }
            Message::CommentPosted(res) => {
                self.comment_posting = false;
                match res {
                    Ok(()) => {
                        self.comment_input.clear();
                        // refetch comments for the open task
                        if let Some(id) = self.open_task.clone() {
                            self.comments_loading = true;
                            let tok = self.notion_token.clone();
                            let id_c = id.clone();
                            return Task::perform(notion::fetch_comments(tok, id), move |res| Message::NotionCommentsFetched(id_c.clone(), res));
                        }
                    }
                    Err(e) => self.notion_error = Some(e),
                }
                Task::none()
            }
            Message::NotionCloseTask => {
                self.open_task = None;
                self.open_task_body = Vec::new();
                self.open_task_comments = Vec::new();
                self.comment_input.clear();
                Task::none()
            }
            Message::TaskOpenDir => {
                Task::perform(async {
                    rfd::AsyncFileDialog::new().set_title("Open directory for this task's session")
                        .pick_folder().await.map(|h| h.path().to_path_buf())
                }, Message::TaskDirPicked)
            }
            Message::TaskNewDir => {
                Task::perform(async {
                    rfd::AsyncFileDialog::new().set_title("Select PARENT folder — new directory will be created inside")
                        .pick_folder().await.map(|h| h.path().to_path_buf())
                }, Message::TaskNewParentPicked)
            }
            Message::TaskDirPicked(path) => {
                if let (Some(p), Some(_)) = (path, self.current_open_task()) {
                    if p.is_dir() {
                        let pi = self.create_or_find_project(p);
                        self.create_session_for_open_task(pi, None);
                    }
                }
                Task::none()
            }
            Message::TaskNewParentPicked(parent) => {
                if let (Some(parent), Some(task)) = (parent, self.current_open_task()) {
                    let base = sanitize_dir_name(&task.title);
                    let folder = if self.date_prefix_enabled {
                        format!("{}-{}", chrono::Local::now().format("%Y-%m-%d"), base)
                    } else { base };
                    let new_path = parent.join(&folder);
                    if std::fs::create_dir_all(&new_path).is_ok() {
                        let pi = self.create_or_find_project(new_path);
                        self.create_session_for_open_task(pi, None);
                    } else {
                        self.notion_error = Some("Could not create directory".into());
                    }
                }
                Task::none()
            }
            Message::SetNewSessionBackend(b) => { self.new_session_backend = b; Task::none() }
            Message::ReopenTaskLink(idx) => {
                if let Some(task) = self.current_open_task() {
                    if let Some(link) = self.task_links.get(&task.id).and_then(|v| v.get(idx)).cloned() {
                        if !link.path.is_dir() {
                            self.notion_error = Some(format!("Folder no longer exists: {}", link.path.display()));
                        } else {
                            let pi = self.create_or_find_project(link.path.clone());
                            self.create_session_for_open_task(pi, Some(link.session_name.clone()));
                        }
                    }
                }
                Task::none()
            }
        }
    }

    fn current_open_task(&self) -> Option<notion::NotionTask> {
        self.open_task.as_ref()
            .and_then(|id| self.notion_tasks.iter().find(|t| &t.id == id))
            .cloned()
    }

    /// Find an existing project by path, or create one. Returns its index.
    fn create_or_find_project(&mut self, path: PathBuf) -> usize {
        if let Some(pi) = self.projects.iter().position(|p| p.path == path) {
            self.active_project = Some(pi);
            return pi;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let mut group = ProjectGroup::new(name, path.clone());
        if let Some(info) = git_info::get_git_info_with_pr(&path) {
            group.branch = info.branch.clone();
            group.dirty_files = info.dirty_files;
            group.sub_repos = to_sub_repo_views(&info);
        }
        self.projects.push(group);
        let pi = self.projects.len() - 1;
        self.active_project = Some(pi);
        self.file_entries = explorer::read_directory(&path, 0);
        pi
    }

    /// Write the open task's markdown into project `pi`, spawn a session, and
    /// remember the (folder, session name) affiliation so it can be reopened.
    /// `name_override` reuses a saved session name (reopen flow).
    fn create_session_for_open_task(&mut self, pi: usize, name_override: Option<String>) {
        let Some(task) = self.current_open_task() else { return; };
        if pi >= self.projects.len() { return; }
        let schema_props = self.notion_schema.as_ref().map(|s| s.props.clone()).unwrap_or_default();
        let mut md = task_to_markdown(&task, &schema_props);
        if !self.open_task_body.is_empty() {
            md.push_str("\n## Description\n\n");
            for b in &self.open_task_body {
                match b.kind.as_str() {
                    "heading_1" | "heading_2" | "heading_3" => md.push_str(&format!("\n### {}\n", b.text)),
                    "to_do" => md.push_str(&format!("- [{}] {}\n", if b.checked == Some(true) {"x"} else {" "}, b.text)),
                    "bulleted_list_item" | "numbered_list_item" => md.push_str(&format!("- {}\n", b.text)),
                    "divider" => md.push_str("\n---\n"),
                    _ => md.push_str(&format!("{}\n", b.text)),
                }
            }
        }
        let stripped: String = task.id.chars().filter(|c| *c != '-').collect();
        let short = &stripped[..stripped.len().min(8)];
        let fname = format!("TASK-{}.md", short);
        let _ = std::fs::write(self.projects[pi].path.join(&fname), md);
        let name = name_override.unwrap_or_else(|| {
            if task.title.trim().is_empty() { fname.clone() } else { task.title.clone() }
        });
        // remember the affiliation (dedupe by path + name)
        let path = self.projects[pi].path.clone();
        let links = self.task_links.entry(task.id.clone()).or_default();
        if !links.iter().any(|l| l.path == path && l.session_name == name) {
            links.push(persistence::TaskLink { path: path.clone(), session_name: name.clone() });
        }
        self.projects[pi].sessions.push(Session::new(name));
        let si = self.projects[pi].sessions.len() - 1;
        // apply the backend picked in the task detail (None if it equals global)
        if self.new_session_backend != self.agent_backend {
            self.projects[pi].sessions[si].backend_override = Some(self.new_session_backend.as_str().to_string());
        }
        self.active_project = Some(pi);
        self.active_session = Some((pi, si));
        self.spawn_session_terminal(pi, si, false);
        self.open_task = None;
        self.screen = Screen::Ide;
        self.save_state();
    }

    // ─── VIEW ───

    fn view(&self) -> Element<'_, Message> {
        let _guard = logging::perf("view");
        let tc = self.tc();
        let center = if let Some((pi, si)) = self.show_pipeline_modal {
            self.view_pipeline_modal(pi, si)
        } else if let Some((pi, ref files)) = self.confirm_delete {
            self.view_confirm_delete(pi, files)
        } else if self.show_settings {
            self.view_settings()
        } else {
            self.view_terminal()
        };

        let tc1 = tc.clone();
        let tc3 = tc.clone();
        let tc4 = tc.clone();
        let tc5 = tc.clone();

        let right: Element<'_, Message> = match self.screen {
            Screen::Ide => row![
                container(center).width(Fill).height(Fill)
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(c(0x17, 0x1b, 0x21))), ..Default::default()
                    }),
                container(self.view_explorer()).width(260).height(Fill)
                    .style(move |_: &Theme| styled_panel(&tc3)),
            ].into(),
            Screen::Board => {
                let inner: Element<'_, Message> = if self.show_settings {
                    self.view_settings()
                } else if let Some(t) = self.open_task.as_ref()
                    .and_then(|id| self.notion_tasks.iter().find(|t| &t.id == id)) {
                    self.view_task_detail(t)
                } else {
                    self.view_board()
                };
                container(inner).width(Fill).height(Fill)
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(c(0x17, 0x1b, 0x21))), ..Default::default()
                    }).into()
            }
        };

        let main = row![
            container(self.view_sessions()).width(self.sidebar_width).height(Fill)
                .style(move |_: &Theme| styled_panel(&tc1)),
            container(right).width(Fill).height(Fill)
                .style(move |_: &Theme| styled_panel(&tc5)),
        ];

        let status = container(
            row![
                text(format!("● {} sessions", self.projects.iter().map(|p| p.sessions.len()).sum::<usize>()))
                    .size(11).color(tc.text_muted),
                if let Some(ref ver) = self.update_available {
                    button(text(format!("⬆ v{} available", ver)).size(10).color(tc.blue))
                        .on_press(Message::OpenUrl("https://github.com/foker/orch-ide/releases".to_string()))
                        .style(button::text).padding([2, 6])
                } else {
                    button(text("").size(1)).style(button::text).padding(0)
                },
                Space::new().width(Fill),
                text(format!("OrchIDE v{}", APP_VERSION)).size(11).color(tc.text_muted),
            ].padding(4).spacing(12),
        ).style(move |_: &Theme| styled_panel(&tc4));

        column![main, status].into()
    }

    fn view_sessions(&self) -> Element<'_, Message> {
        let tc = self.tc();

        let header = container(
            row![
                text("SESSIONS").size(10).font(Font::DEFAULT).color(tc.text_muted),
                Space::new().width(Fill),
                tip(button(text("⟳").size(13).color(tc.text_muted)).on_press(Message::RefreshAll).style(button::text).padding(2), "Refresh git & PR"),
                tip(button(text("◂").size(9).color(tc.text_muted)).on_press(Message::ResizeSidebar(-40.0)).style(button::text).padding(2), "Shrink"),
                tip(button(text("▸").size(9).color(tc.text_muted)).on_press(Message::ResizeSidebar(40.0)).style(button::text).padding(2), "Expand"),
                tip(button(text("⚙").size(13).color(tc.text_muted)).on_press(Message::ToggleSettings).style(button::text).padding(2), "Settings"),
                tip(button(text("⊕").size(13).color(tc.text_muted))
                        .on_press_maybe(self.active_project.filter(|pi| *pi < self.projects.len())
                            .map(|_| Message::NewSessionInCurrentFolder))
                        .style(button::text).padding(2), "New session in current folder (⌘T)"),
                tip(button(text("📁").size(12)).on_press(Message::OpenProject).style(button::text).padding(2), "Open folder (⌘O)"),
                tip(button(text("+").size(14).color(tc.text_muted)).on_press(Message::NewProject).style(button::text).padding(2), "New project (⌘N)"),
            ].spacing(4).align_y(iced::Alignment::Center),
        ).padding([10, 14]);

        let mut content = Column::new().spacing(4).padding(8);

        // New project dialog (when "+" pressed and parent folder selected)
        if self.show_session_dialog == Some(usize::MAX) {
            let check_color = if self.launch_agent { tc.green } else { tc.text_muted };
            let check_icon = if self.launch_agent { "☑" } else { "☐" };
            let parent_name = self.new_project_parent.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            content = content.push(container(column![
                row![
                    text(format!("New project in: {}/", parent_name)).size(11).color(tc.text_secondary),
                    Space::new().width(Fill),
                    tip(
                        button(text("✕").size(11).color(tc.text_muted))
                            .on_press(Message::CancelSessionDialog)
                            .style(button::text).padding([2, 6]),
                        "Cancel"
                    ),
                ].align_y(iced::Alignment::Center),
                text_input("project-name", &self.session_name_input)
                    .id(SESSION_INPUT_ID)
                    .on_input(Message::SessionNameChanged)
                    .on_submit(Message::SessionNameSubmit(usize::MAX, self.session_name_input.clone()))
                    .size(12).padding(6),
                button(
                    row![
                        text(check_icon).size(14).color(check_color),
                        text("Launch Claude").size(11).color(tc.text_secondary),
                    ].spacing(6).align_y(iced::Alignment::Center)
                ).on_press(Message::ToggleLaunchClaude).style(button::text).padding([2, 0]),
                {
                    let mut color_row = Row::new().spacing(4);
                    for &clr in session::SessionColor::all() {
                        let (r, g, b) = clr.to_rgb();
                        let is_sel = self.new_session_color == clr;
                        let dot_size = if is_sel { 12 } else { 10 };
                        color_row = color_row.push(
                            button(text("●").size(dot_size).color(c(r, g, b)))
                                .on_press(Message::SetNewSessionColor(clr))
                                .style(button::text).padding([1, 2])
                        );
                    }
                    color_row
                },
                row![
                    Space::new().width(Fill),
                    button(text("Cancel").size(11).color(tc.text_secondary))
                        .on_press(Message::CancelSessionDialog)
                        .style(button::secondary).padding([4, 12]),
                    Space::new().width(8),
                    button(text("Create").size(11).color(tc.green))
                        .on_press(Message::SessionNameSubmit(usize::MAX, self.session_name_input.clone()))
                        .style(button::secondary).padding([4, 12]),
                ].align_y(iced::Alignment::Center),
            ].spacing(4)).padding(8).style({let t = tc.clone(); move |_: &Theme| styled_card(&t)}));
        }

        // Sort projects by hottest session: AWAIT first, then IDLE, then RUN last
        let mut sorted_project_indices: Vec<usize> = (0..self.projects.len()).collect();
        sorted_project_indices.sort_by_key(|&pi| {
            let best = self.projects[pi].sessions.iter()
                .map(|s| s.sort_key())
                .min()
                .unwrap_or((1, 0)); // default to IDLE if no sessions
            best
        });

        for &pi in &sorted_project_indices {
            let project = &self.projects[pi];
            // No project header — sessions shown directly

            // Session cards (sorted: AWAIT first, IDLE middle, RUN last)
            let mut sorted_indices: Vec<usize> = (0..project.sessions.len()).collect();
            sorted_indices.sort_by_key(|&i| project.sessions[i].sort_key());
            for &si in &sorted_indices {
                let session = &project.sessions[si];
                let is_active = self.active_session == Some((pi, si));
                let sc = tc.status_color(&session.status);
                let t = tc.clone();

                // Top row: dot + name + agents + status + kill btn
                let mut top = Row::new().spacing(6).align_y(iced::Alignment::Center);
                // Pulsing dot for AWAIT status. While a pipeline is running, swap the
                // dot for a gear glyph in the same status colour.
                let dot_color = if session.status == SessionStatus::AwaitingInput && !self.blink_on {
                    Color { a: 0.3, ..sc }
                } else {
                    sc
                };
                if session.pipeline_run.is_some() {
                    top = top.push(text("⚙").size(12).color(dot_color));
                } else {
                    top = top.push(text("●").size(8).color(dot_color));
                }
                // Session name: inline rename if double-clicked, otherwise clickable text
                if self.renaming_session == Some((pi, si)) {
                    top = top.push(
                        text_input("name", &self.rename_input)
                            .on_input(Message::RenameSessionInput)
                            .on_submit(Message::RenameSessionSubmit)
                            .size(13).padding(2).width(Fill)
                    );
                    // Color picker row when renaming
                    // (will be added after meta below)
                } else {
                    top = top.push(text(&session.name).size(13).color(tc.text_primary));
                    // Pencil icon (visible on card hover)
                    if self.hovered_card == Some((pi, si)) || is_active {
                        top = top.push(
                            tip(
                                button(text("✎").size(12).color(Color { a: 0.5, ..tc.text_primary }))
                                    .on_press(Message::StartRenameSession(pi, si))
                                    .style(button::text).padding([0, 3]),
                                "Change title and color"
                            )
                        );
                    }
                }
                if session.background_agents > 0 {
                    top = top.push(text(format!("🤖{}", session.background_agents)).size(10).color(tc.green));
                }
                top = top.push(Space::new().width(Fill));

                // Meta: show sub-repos with branches, or single branch
                let meta: Element<'_, Message> = if project.sub_repos.len() > 1 {
                    let mut col = Column::new().spacing(2);
                    for sr in &project.sub_repos {
                        let label = if sr.name == "." {
                            format!("⎇ {}", sr.branch)
                        } else {
                            format!("{}:⎇ {}", sr.name, sr.branch)
                        };
                        let dir_tip = if sr.name == "." { "Root git repo".to_string() } else { format!("{} dir → git repo", sr.name) };
                        let mut r = Row::new().spacing(4);
                        r = r.push(tip(text(label).size(9).font(MONO_FONT).color(tc.purple), &dir_tip));
                        // PR dot with tooltip
                        let pr_color = if sr.has_unmerged_pr { tc.blue } else { tc.text_muted };
                        let pr_tip = if sr.has_unmerged_pr {
                            format!("Open PR {}", sr.pr_number)
                        } else if !sr.pr_number.is_empty() {
                            format!("PR {} merged", sr.pr_number)
                        } else {
                            "No open PR".to_string()
                        };
                        r = r.push(tip(text("⬤").size(6).color(pr_color), &pr_tip));
                        if sr.dirty_files > 0 {
                            r = r.push(text(format!("· {} files", sr.dirty_files)).size(9).font(MONO_FONT).color(tc.orange));
                        }
                        col = col.push(r);
                    }
                    col.into()
                } else {
                    let mut r = Row::new().spacing(4);
                    r = r.push(text(format!("⎇ {}", project.branch)).size(10).font(MONO_FONT).color(tc.purple));
                    // PR dot with tooltip
                    if let Some(sr) = project.sub_repos.first() {
                        let pr_color = if sr.has_unmerged_pr { tc.blue } else { tc.text_muted };
                        let pr_tip = if sr.has_unmerged_pr {
                            format!("Open PR {}", sr.pr_number)
                        } else if !sr.pr_number.is_empty() {
                            format!("PR {} merged", sr.pr_number)
                        } else {
                            "No open PR".to_string()
                        };
                        r = r.push(tip(text("⬤").size(6).color(pr_color), &pr_tip));
                    }
                    r = r.push(text("·").size(10).color(tc.text_muted));
                    if project.dirty_files > 0 {
                        r = r.push(text(format!("{} files", project.dirty_files)).size(10).font(MONO_FONT).color(tc.orange));
                    } else {
                        r = r.push(tip(text("✓ clean").size(10).font(MONO_FONT).color(tc.text_muted), "No uncommitted changes"));
                    }
                    r.into()
                };

                let (cr, cg, cb) = session.color.to_rgb();
                let session_border_color = c(cr, cg, cb);
                let border_c = if is_active { session_border_color } else {
                    Color { a: 0.4, ..session_border_color }
                };
                let bg_c = if is_active { t.bg_card_hover } else { t.bg_card };
                let hover_bg = t.bg_card_hover;
                // Whole-card bg incl. hover (painted by the container, not the inner button)
                let card_bg = if self.hovered_card == Some((pi, si)) { hover_bg } else { bg_c };

                // Pipeline progress bar (built before the card so it can be included).
                let pipeline_bar: Option<Element<'_, Message>> = session.pipeline_run.as_ref().map(|run| {
                    let total = run.steps.len().max(1);
                    let current = run.current_step.min(total - 1);
                    let label = format!("{} · step {}/{}", run.pipeline_name, current + 1, total);
                    let bar_color = sc;
                    let bg_color = Color { a: 0.15, ..bar_color };
                    let frac = ((current + 1) as f32 / total as f32).clamp(0.0, 1.0);
                    let total_w = (self.sidebar_width - 60.0).max(80.0);
                    let filled_w = (total_w * frac).max(2.0);
                    let bg = container(Space::new().width(filled_w).height(4))
                        .style(move |_: &Theme| container::Style {
                            background: Some(Background::Color(bar_color)),
                            border: Border { radius: 2.0.into(), ..Default::default() },
                            ..Default::default()
                        });
                    let track = container(bg).width(total_w).height(4)
                        .style(move |_: &Theme| container::Style {
                            background: Some(Background::Color(bg_color)),
                            border: Border { radius: 2.0.into(), ..Default::default() },
                            ..Default::default()
                        });
                    column![
                        text(label).size(9).font(MONO_FONT).color(tc.text_secondary),
                        track,
                    ].spacing(3).into()
                });

                // Card content (left, fills) — a transparent select button
                let select_btn = button({
                    let mut card_col = Column::new().spacing(6);
                    card_col = card_col.push(top);
                    card_col = card_col.push(meta);
                    if let Some(pb) = pipeline_bar {
                        card_col = card_col.push(pb);
                    }
                    if self.renaming_session == Some((pi, si)) {
                        let mut color_row = Row::new().spacing(4);
                        for &clr in session::SessionColor::all() {
                            let (r, g, b) = clr.to_rgb();
                            let is_selected = session.color == clr;
                            let dot_size = if is_selected { 12 } else { 10 };
                            color_row = color_row.push(
                                button(text("\u{25CF}").size(dot_size).color(c(r, g, b)))
                                    .on_press(Message::SetSessionColor(pi, si, clr))
                                    .style(button::text).padding([1, 2])
                            );
                        }
                        card_col = card_col.push(color_row);
                    }
                    card_col
                })
                    .on_press(Message::SelectSession(pi, si))
                    .style(move |_: &Theme, _status: button::Status| button::Style {
                        background: None,
                        text_color: Color::WHITE,
                        ..Default::default()
                    })
                    .padding([10, 12])
                    .width(Fill);

                // Faint vertical separator between content and actions
                let sep_color = Color { a: 0.12, ..session_border_color };
                let separator = container(Space::new().width(1).height(Fill))
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(sep_color)),
                        ..Default::default()
                    });

                // Action buttons (right, inside the card)
                let actions = container(column![
                    tip(button(text("\u{2715}").size(11).color(tc.text_muted))
                            .on_press(Message::KillSession(pi, si))
                            .style(button::text).padding([4, 6]),
                        "Kill session"),
                    tip(button(text("\u{25FC}").size(9).color(tc.text_muted))
                            .on_press(Message::MakeIdle(pi, si))
                            .style(button::text).padding([4, 6]),
                        "Set idle"),
                    tip(button(text("\u{25B6}").size(9).color(tc.blue))
                            .on_press(Message::OpenRunPipelineModal(pi, si))
                            .style(button::text).padding([4, 6]),
                        "Run pipeline"),
                    tip(button(text("\u{1F5D1}").size(9))
                            .on_press(Message::DeleteProjectDir(pi))
                            .style(button::text).padding([4, 6]),
                        "Delete folder"),
                ].spacing(0)).padding([4, 2]);

                // Whole card: bordered container holding [content | sep | actions]
                let card_row = container(
                    row![select_btn, separator, actions].align_y(iced::Alignment::Center)
                )
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(card_bg)),
                    border: Border { color: border_c, width: 1.0, radius: 6.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .width(Fill);

                content = content.push(
                    mouse_area(
                        container(card_row)
                            .padding(Padding { top: 2.0, right: 12.0, bottom: 2.0, left: 12.0 })
                    )
                    .on_enter(Message::CardHover(Some((pi, si))))
                    .on_exit(Message::CardHover(None))
                );
            }

            // Add session button or inline input
            if self.show_session_dialog == Some(pi) {
                let check_color = if self.launch_agent { tc.green } else { tc.text_muted };
                let check_icon = if self.launch_agent { "☑" } else { "☐" };
                content = content.push(
                    container(column![
                        text_input("Session name...", &self.session_name_input)
                            .id(SESSION_INPUT_ID)
                            .on_input(Message::SessionNameChanged)
                            .on_submit(Message::SessionNameSubmit(pi, self.session_name_input.clone()))
                            .size(12).padding(6),
                        button(
                            row![
                                text(check_icon).size(14).color(check_color),
                                text("Launch Claude with this session name").size(11).color(tc.text_secondary),
                            ].spacing(6).align_y(iced::Alignment::Center)
                        ).on_press(Message::ToggleLaunchClaude).style(button::text).padding([2, 0]),
                        row![
                            Space::new().width(Fill),
                            button(text("Cancel").size(11).color(tc.text_secondary))
                                .on_press(Message::CancelSessionDialog)
                                .style(button::secondary).padding([4, 12]),
                        ].align_y(iced::Alignment::Center),
                    ].spacing(4))
                    .padding(Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 20.0 })
                );
            }
            // TODO: temporarily hidden
            // } else {
            //     content = content.push(
            //         button(text("+ add session").size(10).color(tc.text_muted))
            //             .on_press(Message::AddSession(pi)).style(button::text).padding([4, 20])
            //     );
            // }
        }

        // Empty state
        if self.projects.is_empty() {
            content = content.push(
                container(
                    column![
                        text("No projects yet").size(13).color(tc.text_muted),
                        text("Click + to add a project folder").size(11).color(tc.text_muted),
                    ].spacing(4).align_x(iced::Alignment::Center)
                ).padding(24).center_x(Fill)
            );
        }

        // Screen switcher (IDE | Board) — persists across both screens
        let seg = |label: &str, target: Screen| {
            let active = self.screen == target;
            button(text(label.to_string()).size(11)
                .color(if active { tc.text_primary } else { tc.text_muted }))
                .on_press(Message::SetScreen(target))
                .style(if active { button::secondary } else { button::text })
                .padding([4, 12])
        };
        let switcher = container(row![
            seg("IDE", Screen::Ide), seg("Board", Screen::Board),
        ].spacing(4)).padding([8, 10]);

        column![switcher, header, rule::horizontal(1), scrollable(content).height(Fill)].into()
    }

    fn view_board(&self) -> Element<'_, Message> {
        let tc = self.tc();

        let Some(schema) = self.notion_schema.as_ref() else {
            return container(column![
                text("No Notion board connected").size(14).color(tc.text_primary),
                text("Open Settings → Notion, paste a token and pick a database.").size(11).color(tc.text_muted),
                button(text("Open Settings").size(12)).on_press(Message::ToggleSettings).style(button::secondary).padding([6,12]),
            ].spacing(10).align_x(iced::Alignment::Center)).center_x(Fill).center_y(Fill).into();
        };

        let gp_id = self.notion_group_by_prop.clone().unwrap_or_default();
        let options: Vec<notion::SelectOption> = schema.props.iter()
            .find(|p| p.id == gp_id)
            .map(|p| match &p.kind {
                notion::PropKind::Status(o) | notion::PropKind::Select(o) => o.clone(),
                _ => vec![],
            }).unwrap_or_default();

        let mut groupby_row = Row::new().spacing(6).align_y(iced::Alignment::Center);
        groupby_row = groupby_row.push(text("Group by:").size(11).color(tc.text_muted));
        for p in schema.props.iter().filter(|p| matches!(p.kind, notion::PropKind::Status(_) | notion::PropKind::Select(_))) {
            let active = self.notion_group_by_prop.as_deref() == Some(p.id.as_str());
            groupby_row = groupby_row.push(
                button(text(p.name.clone()).size(11).color(if active { tc.text_primary } else { tc.text_muted }))
                    .on_press(Message::NotionSetGroupBy(p.id.clone()))
                    .style(if active { button::secondary } else { button::text }).padding([2,8])
            );
        }
        let loading_lbl = if self.notion_loading { " (loading…)" } else { "" };
        let topbar = container(row![
            text(format!("{}{}", schema.title.clone(), loading_lbl)).size(14).color(tc.text_primary),
            Space::new().width(Fill),
            groupby_row,
            button(text("⟳").size(13).color(tc.text_muted)).on_press(Message::NotionRefresh).style(button::text).padding(2),
        ].spacing(12).align_y(iced::Alignment::Center)).padding([10, 16]);

        let err_banner: Element<'_, Message> = if let Some(e) = &self.notion_error {
            container(text(format!("⚠ {e}")).size(11).color(tc.red)).padding([4,16]).into()
        } else { Space::new().height(0).into() };

        let cols = group_tasks(&self.notion_tasks, &gp_id, &options);
        let mut board_row = Row::new().spacing(12).padding(12);
        for (label, tasks) in cols {
            // column accent color from the matching Notion option (gray for "(none)")
            let accent = options.iter().find(|o| o.name == label)
                .map(|o| notion_color(&o.color)).unwrap_or_else(|| c(0x9b, 0xa1, 0xa6));
            let mut col = Column::new().spacing(8).padding(8);
            let header_pill = container(
                text(format!("{}  {}", label, tasks.len())).size(11).color(c(0x1a, 0x1a, 0x1a))
            ).padding([2, 8]).style(move |_: &Theme| container::Style {
                background: Some(Background::Color(accent)),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            });
            let add_label = label.clone();
            col = col.push(row![
                header_pill,
                Space::new().width(Fill),
                tip(button(text("+").size(14).color(tc.text_muted))
                        .on_press(Message::BoardAddTaskStart(add_label))
                        .style(button::text).padding([0, 4]),
                    "Add task to this column"),
            ].align_y(iced::Alignment::Center));
            // inline new-task input for this column
            if self.adding_task_col.as_deref() == Some(label.as_str()) {
                col = col.push(
                    column![
                        text_input("New task title…", &self.new_task_title)
                            .on_input(Message::BoardAddTaskInput)
                            .on_submit(Message::BoardAddTaskSubmit)
                            .size(12).padding(6),
                        row![
                            button(text(if self.creating_task { "Adding…" } else { "Add" }).size(11).color(tc.green))
                                .on_press(Message::BoardAddTaskSubmit).style(button::text).padding([2, 6]),
                            button(text("Cancel").size(11).color(tc.text_muted))
                                .on_press(Message::BoardAddTaskCancel).style(button::text).padding([2, 6]),
                        ].spacing(4),
                    ].spacing(4)
                );
            }
            for t in tasks {
                let tid = t.id.clone();
                let card = button(
                    column![ text(t.title.clone()).size(12).color(tc.text_primary) ].spacing(4)
                ).on_press(Message::NotionOpenTask(tid)).style(button::secondary).padding(10).width(Fill);
                col = col.push(card);
            }
            let tc_col = tc.clone();
            board_row = board_row.push(
                container(scrollable(col)).width(240).height(Fill)
                    .style(move |_: &Theme| styled_panel(&tc_col))
            );
        }

        column![
            topbar, err_banner,
            scrollable(board_row)
                .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new()))
                .height(Fill),
        ].into()
    }

    fn view_task_detail(&self, task: &notion::NotionTask) -> Element<'_, Message> {
        let tc = self.tc();
        let schema = self.notion_schema.as_ref();
        let header = container(row![
            text(task.title.clone()).size(15).color(tc.text_primary),
            Space::new().width(Fill),
            button(text("Open in Notion").size(10).color(tc.blue))
                .on_press(Message::OpenUrl(task.url.clone())).style(button::text).padding([2,6]),
            button(text("✕").size(14).color(tc.text_muted)).on_press(Message::NotionCloseTask).style(button::text).padding(4),
        ].align_y(iced::Alignment::Center)).padding([14,18]);

        let mut props_col = Column::new().spacing(8).padding([8,18]);
        if let Some(sch) = schema {
            for p in &sch.props {
                if matches!(p.kind, notion::PropKind::Title) { continue; }
                let rendered = match task.props.get(&p.id) {
                    Some(notion::PropValue::Text(s)) => s.clone(),
                    Some(notion::PropValue::Number(n)) => n.to_string(),
                    Some(notion::PropValue::Checkbox(b)) => if *b {"☑".into()} else {"☐".into()},
                    Some(notion::PropValue::Url(u)) => u.clone(),
                    Some(notion::PropValue::Date(d)) => d.clone(),
                    Some(notion::PropValue::Select(Some(o))) => o.name.clone(),
                    Some(notion::PropValue::MultiSelect(v)) => v.iter().map(|o| o.name.clone()).collect::<Vec<_>>().join(", "),
                    Some(notion::PropValue::People(v)) => v.join(", "),
                    Some(notion::PropValue::Raw(_)) => "—".into(),
                    _ => "".into(),
                };
                props_col = props_col.push(row![
                    text(p.name.clone()).size(11).color(tc.text_muted).width(160),
                    text(rendered).size(12).color(tc.text_secondary).width(Fill),
                ].spacing(8));
            }
        }

        // ── Body / description (page content blocks) ──
        let mut body_col = Column::new().spacing(6).padding([8, 24]);
        if self.open_task_body_loading {
            body_col = body_col.push(text("Loading description…").size(11).color(tc.text_muted));
        } else if self.open_task_body.is_empty() {
            body_col = body_col.push(text("No description.").size(12).color(tc.text_muted));
        } else {
            for b in &self.open_task_body {
                let el: Element<'_, Message> = match b.kind.as_str() {
                    "heading_1" => text(b.text.clone()).size(16).color(tc.text_primary).into(),
                    "heading_2" => text(b.text.clone()).size(14).color(tc.text_primary).into(),
                    "heading_3" => text(b.text.clone()).size(13).color(tc.text_primary).into(),
                    "to_do" => row![
                        text(if b.checked == Some(true) {"☑"} else {"☐"}).size(12)
                            .color(if b.checked == Some(true) { tc.green } else { tc.text_muted }),
                        text(b.text.clone()).size(12).color(tc.text_secondary).width(Fill),
                    ].spacing(6).into(),
                    "bulleted_list_item" => row![
                        text("•").size(12).color(tc.text_muted),
                        text(b.text.clone()).size(12).color(tc.text_secondary).width(Fill),
                    ].spacing(6).into(),
                    "numbered_list_item" => row![
                        text("–").size(12).color(tc.text_muted),
                        text(b.text.clone()).size(12).color(tc.text_secondary).width(Fill),
                    ].spacing(6).into(),
                    "quote" => row![
                        text("▎").size(12).color(tc.text_muted),
                        text(b.text.clone()).size(12).color(tc.text_secondary).width(Fill),
                    ].spacing(6).into(),
                    "code" => text(b.text.clone()).size(11).font(MONO_FONT).color(tc.text_secondary).into(),
                    "divider" => rule::horizontal(1).into(),
                    _ => text(b.text.clone()).size(12).color(tc.text_secondary).into(),
                };
                body_col = body_col.push(el);
            }
        }

        // ── Footer: create / reopen a session affiliated with this task ──
        let green = tc.green;
        let mut footer = Column::new().spacing(8).padding([12, 18]);
        // previously-affiliated folders → reopen
        if let Some(links) = self.task_links.get(&task.id) {
            if !links.is_empty() {
                footer = footer.push(text("Affiliated folders").size(11).color(tc.text_muted));
                for (i, link) in links.iter().enumerate() {
                    footer = footer.push(
                        button(row![
                            text("↻").size(12).color(green),
                            column![
                                text(link.session_name.clone()).size(12).color(tc.text_primary),
                                text(link.path.display().to_string()).size(9).color(tc.text_muted),
                            ].spacing(1),
                        ].spacing(8).align_y(iced::Alignment::Center))
                        .on_press(Message::ReopenTaskLink(i))
                        .style(button::secondary).padding([6, 10]).width(Fill)
                    );
                }
            }
        }
        let action_btn = |label: &str, msg: Message| {
            button(container(text(label.to_string()).size(12).color(tc.text_primary)).center_x(Fill).padding([5, 0]))
                .on_press(msg).style(button::secondary).width(Fill)
        };
        footer = footer.push(text("New session from this task").size(11).color(tc.text_muted));
        // Model picker for the new session (defaults to the global settings backend)
        let mut model_row = Row::new().spacing(6).align_y(iced::Alignment::Center);
        model_row = model_row.push(text("Model:").size(10).color(tc.text_muted));
        for &b in AgentBackend::all() {
            let active = self.new_session_backend == b;
            model_row = model_row.push(
                button(text(b.label()).size(10).color(if active { tc.text_primary } else { tc.text_muted }))
                    .on_press(Message::SetNewSessionBackend(b))
                    .style(if active { button::secondary } else { button::text }).padding([2, 8])
            );
        }
        footer = footer.push(model_row);
        footer = footer.push(row![
            action_btn("📂 Open directory", Message::TaskOpenDir),
            action_btn("＋ Create new directory", Message::TaskNewDir),
        ].spacing(8));

        // Comments section (under the description, left column)
        body_col = body_col.push(Space::new().height(16));
        body_col = body_col.push(rule::horizontal(1));
        body_col = body_col.push(text(format!("COMMENTS ({})", self.open_task_comments.len())).size(10).color(tc.text_muted));
        if self.comments_loading {
            body_col = body_col.push(text("Loading comments…").size(11).color(tc.text_muted));
        } else if self.open_task_comments.is_empty() {
            body_col = body_col.push(text("No comments yet.").size(11).color(tc.text_muted));
        } else {
            for cm in &self.open_task_comments {
                let when = cm.created_time.split('T').next().unwrap_or("").to_string();
                body_col = body_col.push(container(column![
                    text(cm.text.clone()).size(12).color(tc.text_secondary),
                    text(when).size(9).color(tc.text_muted),
                ].spacing(2)).padding([6, 0]));
            }
        }
        body_col = body_col.push(
            row![
                text_input("Write a comment…", &self.comment_input)
                    .on_input(Message::CommentInput)
                    .on_submit(Message::PostComment)
                    .size(12).padding(8),
                button(text(if self.comment_posting { "…" } else { "Send" }).size(11).color(tc.blue))
                    .on_press(Message::PostComment).style(button::secondary).padding([6, 12]),
            ].spacing(8).align_y(iced::Alignment::Center)
        );

        // Two-column body: description (left, main) + properties (right sidebar)
        let tcp = tc.clone();
        let body = row![
            container(scrollable(body_col).height(Fill)).width(Fill).height(Fill),
            container(scrollable(props_col).height(Fill)).width(360).height(Fill)
                .style(move |_: &Theme| styled_panel(&tcp)),
        ].height(Fill);

        column![
            header, rule::horizontal(1),
            body,
            rule::horizontal(1),
            footer,
        ].into()
    }

    fn view_terminal(&self) -> Element<'_, Message> {
        let tc = self.tc();

        // Validate active_session: if indices are stale (e.g. after deletion races),
        // render the empty state instead of indexing OOB and aborting via panic_cannot_unwind
        // inside the macOS event-loop block.
        let (pi, si) = match self.active_session {
            Some((pi, si))
                if pi < self.projects.len() && si < self.projects[pi].sessions.len() =>
            {
                (pi, si)
            }
            _ => {
                return container(
                    column![
                        text("◆").size(32).color(tc.text_muted),
                        text("Select or create a session").size(14).color(tc.text_muted),
                    ].spacing(8).align_x(iced::Alignment::Center)
                ).center(Fill).height(Fill).into();
            }
        };
        let project = &self.projects[pi];
        let session = &project.sessions[si];
        let sc = tc.status_color(&session.status);

        // Info bar with chips
        let status_text = match session.status {
            SessionStatus::AwaitingInput => "● Awaiting input",
            SessionStatus::Running => "● Running",
            SessionStatus::Done => "● Done",
            SessionStatus::Error => "● Error",
            SessionStatus::Idle => "● Idle",
        };

        let t = tc.clone();

        // Status + session name + path row
        let mut top_row = Row::new().spacing(8).align_y(iced::Alignment::Center);
        top_row = top_row.push(chip(status_text, sc));
        let is_refreshing = self.refresh_pending_git.contains(&pi) || self.refresh_pending_deps.contains(&pi);
        let (refresh_glyph, refresh_color) = if is_refreshing {
            // Animated spinner: cycle through frames via blink_on / tick_count
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame = frames[(self.tick_count as usize) % frames.len()];
            (frame.to_string(), tc.blue)
        } else {
            ("⟳".to_string(), tc.text_muted)
        };
        let mut refresh_btn = button(text(refresh_glyph).size(10).color(refresh_color))
            .style(button::text).padding([2, 4]);
        if !is_refreshing {
            refresh_btn = refresh_btn.on_press(Message::RefreshProject(pi));
        }
        let tip_label = if is_refreshing { "Refreshing…" } else { "Refresh git & deployments" };
        top_row = top_row.push(tip(refresh_btn, tip_label));

        // Affiliated Notion task → compact header card (click → Board + open task)
        let sess_name = self.projects[pi].sessions[si].name.clone();
        let sess_path = self.projects[pi].path.clone();
        let affiliated = self.task_links.iter().find_map(|(tid, links)| {
            if links.iter().any(|l| l.path == sess_path && l.session_name == sess_name) {
                let title = self.notion_tasks.iter().find(|t| &t.id == tid)
                    .map(|t| t.title.clone())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| sess_name.clone());
                Some((tid.clone(), title))
            } else { None }
        });
        if let Some((tid, title)) = affiliated {
            let short: String = if title.chars().count() > 42 {
                format!("{}…", title.chars().take(42).collect::<String>())
            } else { title };
            let tcc = tc.clone();
            top_row = top_row.push(
                button(row![
                    text("📋").size(10),
                    text(short).size(10).color(tc.text_secondary),
                ].spacing(5).align_y(iced::Alignment::Center))
                .on_press(Message::OpenTaskBoard(tid))
                .style(move |_: &Theme, _| button::Style {
                    background: Some(Background::Color(Color { a: 0.05, ..Color::WHITE })),
                    text_color: tcc.text_secondary,
                    border: Border { color: tcc.border, width: 1.0, radius: 5.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding([3, 8])
            );
        }

        top_row = top_row.push(Space::new().width(Fill));
        top_row = top_row.push(text(project.path.display().to_string()).size(9).font(MONO_FONT).color(tc.text_muted));

        // Per-branch cards
        let mut branch_cards = Row::new().spacing(6);
        for sr in &project.sub_repos {
            let t2 = tc.clone();
            let mut card_col = Column::new().spacing(2);

            // Row 1: folder + branch
            let folder_label = if sr.name == "." { "" } else { &sr.name };
            card_col = card_col.push(
                row![
                    text(format!("{} ⎇ {}", folder_label, sr.branch)).size(11).font(MONO_FONT).color(tc.purple),
                    text(&sr.commit_short).size(9).font(MONO_FONT).color(tc.text_muted),
                ].spacing(4)
            );

            // Row 2: stats
            let mut stats = Row::new().spacing(4);
            if sr.dirty_files > 0 {
                stats = stats.push(text(format!("✎{}", sr.dirty_files)).size(11).font(MONO_FONT).color(tc.orange));
            }
            if sr.unpushed_commits > 0 {
                stats = stats.push(text(format!("⬆{}", sr.unpushed_commits)).size(11).font(MONO_FONT).color(tc.yellow));
            }
            if sr.dirty_files == 0 && sr.unpushed_commits == 0 {
                stats = stats.push(text("✓").size(11).color(tc.text_muted));
            }
            // PR
            if !sr.pr_number.is_empty() {
                let pr_color = if sr.has_unmerged_pr { tc.blue } else { tc.text_muted };
                if !sr.pr_url.is_empty() {
                    let url = sr.pr_url.clone();
                    stats = stats.push(
                        button(text(&sr.pr_number).size(11).font(MONO_FONT).color(pr_color))
                            .on_press(Message::OpenUrl(url))
                            .style(button::text).padding(0)
                    );
                } else {
                    stats = stats.push(text(&sr.pr_number).size(11).font(MONO_FONT).color(pr_color));
                }
            }
            // Deployment
            if let Some(dep) = sr.deployments.first() {
                let dep_icon = match dep.state.as_str() {
                    "success" => "🟢", "failure" | "error" => "🔴",
                    "in_progress" | "pending" => "⏳", _ => "⚪",
                };
                if !dep.url.is_empty() {
                    let url = dep.url.clone();
                    let short = dep.url.replace("https://", "").chars().take(30).collect::<String>();
                    stats = stats.push(
                        button(text(format!("{}{}", dep_icon, short)).size(10).font(MONO_FONT).color(tc.blue))
                            .on_press(Message::OpenUrl(url))
                            .style(button::text).padding(0)
                    );
                } else {
                    stats = stats.push(text(dep_icon).size(11));
                }
            }
            // Local dev servers
            for ds in &sr.dev_servers {
                let icon = if ds.running { "🖥" } else { "⬜" };
                let color = if ds.running { tc.green } else { tc.text_muted };
                let short = ds.url.replace("http://", "").replace("https://", "");
                let url = ds.url.clone();
                stats = stats.push(
                    button(text(format!("{}{}", icon, short)).size(10).font(MONO_FONT).color(color))
                        .on_press(Message::OpenUrl(url))
                        .style(button::text).padding(0)
                );
            }
            card_col = card_col.push(stats);

            branch_cards = branch_cards.push(
                container(card_col).padding([4, 8])
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(Color { a: 0.03, ..Color::WHITE })),
                        border: Border { color: t2.border, width: 1.0, radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    })
            );
        }

        let info_bar = container(
            column![
                top_row,
                scrollable(branch_cards).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())).height(50),
            ].spacing(4).padding([6, 12])
        ).style(move |_: &Theme| container::Style {
            background: Some(Background::Color(t.bg_panel)),
            border: Border { color: t.border, width: 0.0, ..Default::default() },
            ..Default::default()
        });

        // Deployments now shown inline in branch cards above

        // Terminal widget (hide when session dialog is open to prevent keyboard conflict)
        let terminal_view: Element<'_, Message> = if self.show_session_dialog.is_some() {
            container(text("").size(1)).height(Fill).into()
        } else if let Some((_, ti)) = self.terminals.iter().find(|(k, _)| *k == (pi, si)) {
            container(
                iced_term::TerminalView::show(&ti.terminal).map(Message::TermEvent)
            ).padding([4, 8]).height(Fill).into()
        } else {
            container(text("No terminal").size(13).color(tc.text_muted)).center(Fill).height(Fill).into()
        };

        // Voice button bar at bottom
        let voice_icon = if self.voice_transcribing {
            "⏳"
        } else if self.voice_recorder.is_recording {
            if self.blink_on { "⏺" } else { "🎙" }
        } else {
            "🎙"
        };
        let voice_color = if self.voice_recorder.is_recording { tc.red } else { tc.text_muted };
        let voice_label = if self.voice_transcribing {
            "Transcribing..."
        } else if self.voice_recorder.is_recording {
            "Recording... click to stop"
        } else {
            ""
        };

        // Bottom bar: quick prompts (left) + voice (right)
        let mut bottom_row = Row::new().spacing(4).padding([3, 8]).align_y(iced::Alignment::Center);
        for prompt in &self.quick_prompts {
            let p = prompt.clone();
            // Count chars, not bytes — truncating at a byte index in the middle of a multi-byte
            // char (e.g. Cyrillic) panics and takes the app down via the winit event-loop block.
            let short = if prompt.chars().count() > 25 {
                let truncated: String = prompt.chars().take(22).collect();
                format!("{}...", truncated)
            } else {
                prompt.clone()
            };
            bottom_row = bottom_row.push(
                button(text(short).size(9).color(tc.text_secondary))
                    .on_press(Message::SendQuickPrompt(p))
                    .style(move |_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                            _ => Color::from_rgba(1.0, 1.0, 1.0, 0.03),
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.08), width: 1.0, radius: 10.0.into(), ..Default::default() },
                            text_color: Color::WHITE,
                            ..Default::default()
                        }
                    })
                    .padding([3, 10])
            );
        }
        bottom_row = bottom_row.push(Space::new().width(Fill));
        bottom_row = bottom_row.push(
            tip(
                button(text(voice_icon).size(16).color(voice_color))
                    .on_press(Message::VoiceToggle)
                    .style(button::text).padding([4, 8]),
                "Voice input"
            )
        );
        if !voice_label.is_empty() {
            bottom_row = bottom_row.push(text(voice_label).size(10).color(tc.text_muted));
        }
        let voice_bar = container(bottom_row);

        // Pipeline control bar (only when this session has an active pipeline run).
        let pipeline_bar: Option<Element<'_, Message>> = session.pipeline_run.as_ref().map(|run| {
            let mut bar = Row::new().spacing(8).align_y(iced::Alignment::Center).padding([4, 12]);
            bar = bar.push(text(format!("⚙ {}", run.pipeline_name)).size(11).font(MONO_FONT).color(tc.text_secondary));
            // Step chips
            let mut chips = Row::new().spacing(4);
            for (idx, step) in run.steps.iter().enumerate() {
                let is_current = idx == run.current_step;
                let is_done = idx < run.current_step;
                let (label_color, bg_color, border_color) = if is_current {
                    (tc.text_primary, Color { a: 0.18, ..tc.blue }, tc.blue)
                } else if is_done {
                    (tc.text_muted, Color { a: 0.05, ..tc.green }, Color { a: 0.4, ..tc.green })
                } else {
                    (tc.text_muted, Color { a: 0.04, ..Color::WHITE }, Color { a: 0.15, ..Color::WHITE })
                };
                let icon = if step.interactive { "👤" } else { "▸" };
                let tip_text = format!(
                    "Step {}: {}{}\nClick to restart from this step.",
                    idx + 1,
                    if step.prompt.chars().count() > 60 {
                        let s: String = step.prompt.chars().take(57).collect();
                        format!("{}...", s)
                    } else { step.prompt.clone() },
                    if step.interactive { " (interactive)" } else { "" },
                );
                chips = chips.push(tip(
                    button(row![
                        text(icon).size(10),
                        text(format!("{}", idx + 1)).size(10).font(MONO_FONT).color(label_color),
                    ].spacing(3).align_y(iced::Alignment::Center))
                        .on_press(Message::PipelineJumpToStep(pi, si, idx))
                        .style(move |_: &Theme, _: button::Status| button::Style {
                            background: Some(Background::Color(bg_color)),
                            border: Border { color: border_color, width: 1.0, radius: 8.0.into(), ..Default::default() },
                            text_color: label_color,
                            ..Default::default()
                        })
                        .padding([2, 6]),
                    &tip_text,
                ));
            }
            bar = bar.push(chips);
            bar = bar.push(Space::new().width(Fill));
            // Next-step button (useful especially for interactive steps that don't auto-advance).
            let has_next = run.current_step + 1 < run.steps.len();
            let next_color = if has_next { tc.green } else { tc.text_muted };
            let mut next_btn = button(text("▶ next step").size(10).color(next_color))
                .style(button::text).padding([2, 6]);
            if has_next { next_btn = next_btn.on_press(Message::PipelineNextStep(pi, si)); }
            bar = bar.push(tip(next_btn, "Advance to next step (kills current claude, spawns next)"));
            // Restart from beginning
            bar = bar.push(tip(
                button(text("↻ restart").size(10).color(tc.yellow))
                    .on_press(Message::PipelineJumpToStep(pi, si, 0))
                    .style(button::text).padding([2, 6]),
                "Restart pipeline from step 1",
            ));
            // Stop pipeline (leaves current claude alive)
            bar = bar.push(tip(
                button(text("✕ stop").size(10).color(tc.red))
                    .on_press(Message::PipelineStop(pi, si))
                    .style(button::text).padding([2, 6]),
                "Stop driving the pipeline (keeps current claude session alive)",
            ));
            let bar_tc = tc.clone();
            container(bar).style(move |_: &Theme| container::Style {
                background: Some(Background::Color(Color { a: 0.04, ..Color::WHITE })),
                border: Border { color: bar_tc.border, width: 0.0, ..Default::default() },
                ..Default::default()
            }).into()
        });

        let mut main_col = Column::new();
        main_col = main_col.push(info_bar);
        if let Some(pb) = pipeline_bar {
            main_col = main_col.push(rule::horizontal(1));
            main_col = main_col.push(pb);
        }
        main_col = main_col.push(rule::horizontal(1));
        main_col = main_col.push(terminal_view);
        main_col = main_col.push(voice_bar);
        main_col.into()
    }

    fn view_explorer(&self) -> Element<'_, Message> {
        let tc = self.tc();

        let header = container(
            row![
                text("EXPLORER").size(10).color(tc.text_muted),
                Space::new().width(Fill),
                button(text("⟳").size(12).color(tc.text_muted)).on_press(Message::RefreshExplorer).style(button::text).padding(2),
            ].align_y(iced::Alignment::Center),
        ).padding([10, 14]);

        let mut tree = Column::new().spacing(1).padding(8);
        for (idx, entry) in self.file_entries.iter().enumerate() {
            let indent = (entry.depth * 16) as u16;
            let (icon, color) = match &entry.kind {
                explorer::FileEntryKind::Directory => (if entry.expanded { "▾" } else { "▸" }, tc.blue),
                explorer::FileEntryKind::File => ("─", tc.text_secondary),
            };
            // Git status coloring
            let color = match entry.git_status {
                explorer::GitStatus::Modified => tc.orange,
                explorer::GitStatus::Added | explorer::GitStatus::Untracked => tc.green,
                explorer::GitStatus::Deleted => tc.red,
                _ => color,
            };
            let msg = match entry.kind {
                explorer::FileEntryKind::Directory => Message::ToggleFileExpand(idx),
                explorer::FileEntryKind::File => Message::OpenFile(entry.path.clone()),
            };
            tree = tree.push(
                button(row![
                    text(icon).size(12).font(MONO_FONT).color(color),
                    text(&entry.name).size(12).font(MONO_FONT).color(color),
                ].spacing(6))
                .on_press(msg).style(button::text)
                .padding([2, 6 + indent])
            );
        }

        column![header, rule::horizontal(1), scrollable(tree).height(Fill)].into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let tc = self.tc();

        let header = container(
            row![
                text("SETTINGS").size(14).color(tc.text_primary),
                Space::new().width(Fill),
                button(text("✕").size(14).color(tc.text_muted)).on_press(Message::ToggleSettings).style(button::text).padding(4),
            ].align_y(iced::Alignment::Center),
        ).padding([16, 20]);

        let mut themes = Column::new().spacing(6);
        for theme in AppTheme::all() {
            let is_active = *theme == self.current_theme;
            let thc = theme.colors();

            let swatch_bg = thc.bg_deep;
            let swatch_border = if is_active { thc.border_active } else { thc.border };
            let swatch = container(Space::new().width(24).height(24))
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(swatch_bg)),
                    border: Border { color: swatch_border, width: if is_active { 2.0 } else { 1.0 }, radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                });

            let label = text(theme.name()).size(13).color(if is_active { tc.text_primary } else { tc.text_secondary });
            let check = text(if is_active { "✓" } else { "" }).size(13).color(tc.green);

            themes = themes.push(
                button(row![swatch, label, Space::new().width(Fill), check].spacing(12).align_y(iced::Alignment::Center))
                    .on_press(Message::SetTheme(theme.clone()))
                    .style(if is_active { button::secondary } else { button::text })
                    .padding([8, 12]).width(Fill),
            );
        }

        // Options section
        let options_section = column![
            // Claude permissions
            {
                let check = if self.dangerously_skip_permissions { "☑" } else { "☐" };
                let check_color = if self.dangerously_skip_permissions { tc.green } else { tc.text_muted };
                button(row![
                    text(check).size(14).color(check_color),
                    text("Launch with --dangerously-skip-permissions").size(11).color(tc.text_secondary),
                ].spacing(6).align_y(iced::Alignment::Center))
                .on_press(Message::ToggleDangerouslySkipPermissions)
                .style(button::text).padding([4, 0])
            },
            // Date prefix toggle
            {
                let check = if self.date_prefix_enabled { "☑" } else { "☐" };
                let check_color = if self.date_prefix_enabled { tc.green } else { tc.text_muted };
                button(row![
                    text(check).size(14).color(check_color),
                    text("Add date prefix to new project folders (YYYY-MM-DD-)").size(11).color(tc.text_secondary),
                ].spacing(6).align_y(iced::Alignment::Center))
                .on_press(Message::ToggleDatePrefix)
                .style(button::text).padding([4, 0])
            },
            Space::new().height(12),
            text("Voice Input (Groq Whisper)").size(12).color(tc.text_muted),
            text_input("Groq API key...", &self.groq_api_key)
                .on_input(Message::GroqKeyChanged)
                .size(12).padding(6),
            text("Get key at console.groq.com").size(10).color(tc.text_muted),
        ].spacing(4);

        // Quick prompts section
        let mut qp_section = Column::new().spacing(4);
        qp_section = qp_section.push(text("Quick Prompts").size(12).color(tc.text_muted));
        for (i, prompt) in self.quick_prompts.iter().enumerate() {
            qp_section = qp_section.push(
                row![
                    text(prompt.clone()).size(11).color(tc.text_secondary).width(Fill),
                    button(text("✕").size(9).color(tc.text_muted))
                        .on_press(Message::RemoveQuickPrompt(i))
                        .style(button::text).padding([2, 4]),
                ].align_y(iced::Alignment::Center)
            );
        }
        qp_section = qp_section.push(
            row![
                text_input("New quick prompt...", &self.quick_prompt_input)
                    .on_input(Message::QuickPromptInput)
                    .on_submit(Message::AddQuickPrompt)
                    .size(11).padding(4),
                button(text("+").size(12).color(tc.text_muted))
                    .on_press(Message::AddQuickPrompt)
                    .style(button::text).padding([2, 6]),
            ].spacing(4).align_y(iced::Alignment::Center)
        );

        // Agent backend select
        let mut backend_row = Row::new().spacing(6);
        for &b in AgentBackend::all() {
            let active = self.agent_backend == b;
            backend_row = backend_row.push(
                button(text(b.label()).size(11)
                    .color(if active { tc.text_primary } else { tc.text_muted }))
                    .on_press(Message::SetAgentBackend(b))
                    .style(if active { button::secondary } else { button::text }).padding([4,12])
            );
        }
        let agent_section = column![
            text("Agent Backend").size(12).color(tc.text_muted),
            backend_row,
            text("Only Claude Code has live status tracking (hooks). OpenCode / Codex /").size(9).color(tc.text_muted),
            text("Xiaomi MiMo run their own TUI; MiMo = opencode pinned to mimo-v2.5-pro.").size(9).color(tc.text_muted),
        ].spacing(6);

        // Notion section
        let mut notion_section = Column::new().spacing(6);
        notion_section = notion_section.push(text("Notion").size(12).color(tc.text_muted));
        notion_section = notion_section.push(
            text_input("Notion integration token (secret_...)", &self.notion_token)
                .on_input(Message::NotionTokenChanged)
                .on_submit(Message::NotionConnect)
                .size(12).padding(6)
        );
        notion_section = notion_section.push(
            button(text(if self.notion_loading { "Connecting…" } else { "Connect / List databases" }).size(11).color(tc.blue))
                .on_press(Message::NotionConnect).style(button::text).padding([2,0])
        );
        if !self.notion_databases.is_empty() {
            let mut dbs = Column::new().spacing(4);
            for db in &self.notion_databases {
                let active = self.notion_database_id.as_deref() == Some(db.id.as_str());
                dbs = dbs.push(
                    button(row![
                        text(if active {"●"} else {"○"}).size(11).color(if active { tc.green } else { tc.text_muted }),
                        text(db.title.clone()).size(11).color(tc.text_secondary),
                    ].spacing(6))
                    .on_press(Message::NotionSelectDatabase(db.id.clone()))
                    .style(button::text).padding([2,0])
                );
            }
            notion_section = notion_section.push(dbs);
        }
        if let Some(e) = &self.notion_error {
            notion_section = notion_section.push(text(format!("⚠ {e}")).size(10).color(tc.red));
        }

        // Footer
        let footer = container(
            column![
                text("all changes saved automatically").size(10).color(tc.text_muted),
                button(text("close").size(11).color(tc.blue))
                    .on_press(Message::ToggleSettings)
                    .style(button::text).padding(0),
            ].spacing(2).align_x(iced::Alignment::End)
        ).padding([8, 20]).width(Fill);

        // Pipelines section
        let mut pl_section = Column::new().spacing(6);
        pl_section = pl_section.push(text("Pipelines").size(12).color(tc.text_muted));
        pl_section = pl_section.push(
            text("Define ordered prompt sequences. \"Run pipeline\" on a card writes its requirements to <project>/.orchpipeline and steps run in order.")
                .size(10).color(tc.text_muted)
        );
        for (idx, p) in self.pipelines.iter().enumerate() {
            let expanded = self.editing_pipeline == Some(idx);
            let chevron = if expanded { "▾" } else { "▸" };
            let mut card = Column::new().spacing(6);
            card = card.push(
                row![
                    button(text(chevron).size(11).color(tc.text_muted))
                        .on_press(Message::ToggleEditPipeline(idx))
                        .style(button::text).padding([2, 6]),
                    text_input("pipeline name", &p.name)
                        .on_input(move |s| Message::PipelineNameChanged(idx, s))
                        .size(12).padding(4),
                    text(format!("{} step{}", p.steps.len(), if p.steps.len() == 1 { "" } else { "s" }))
                        .size(10).color(tc.text_muted),
                    button(text("✕").size(10).color(tc.text_muted))
                        .on_press(Message::RemovePipeline(idx))
                        .style(button::text).padding([2, 6]),
                ].spacing(6).align_y(iced::Alignment::Center)
            );
            if expanded {
                let mut steps_col = Column::new().spacing(4).padding([4, 16]);
                steps_col = steps_col.push(
                    text("Prepend to each step's prompt").size(10).color(tc.text_muted)
                );
                steps_col = steps_col.push(
                    text_input("prepend (use {requirements} for the .orchpipeline path)", &p.prepend_template)
                        .on_input(move |s| Message::PipelinePrependChanged(idx, s))
                        .size(11).padding(4)
                );
                steps_col = steps_col.push(
                    text("{requirements} is replaced with the absolute path to <project>/.orchpipeline at run time. Leave blank to disable prepending.")
                        .size(9).color(tc.text_muted)
                );
                steps_col = steps_col.push(Space::new().height(8));
                let last_idx = p.steps.len().saturating_sub(1);
                for (s_idx, step) in p.steps.iter().enumerate() {
                    let check = if step.interactive { "☑" } else { "☐" };
                    let check_color = if step.interactive { tc.green } else { tc.text_muted };
                    let can_up = s_idx > 0;
                    let can_down = s_idx < last_idx;
                    let up_color = if can_up { tc.text_secondary } else { Color { a: 0.3, ..tc.text_muted } };
                    let down_color = if can_down { tc.text_secondary } else { Color { a: 0.3, ..tc.text_muted } };
                    let mut up_btn = button(text("↑").size(11).color(up_color))
                        .style(button::text).padding([2, 4]);
                    if can_up { up_btn = up_btn.on_press(Message::MovePipelineStep(idx, s_idx, -1)); }
                    let mut down_btn = button(text("↓").size(11).color(down_color))
                        .style(button::text).padding([2, 4]);
                    if can_down { down_btn = down_btn.on_press(Message::MovePipelineStep(idx, s_idx, 1)); }
                    steps_col = steps_col.push(
                        row![
                            column![
                                tip(up_btn, "Move step up"),
                                tip(down_btn, "Move step down"),
                            ].spacing(0),
                            text(format!("{}.", s_idx + 1)).size(10).color(tc.text_muted),
                            text_input("prompt for this step", &step.prompt)
                                .on_input(move |s| Message::PipelineStepPromptChanged(idx, s_idx, s))
                                .size(11).padding(4),
                            tip(
                                button(row![
                                    text(check).size(12).color(check_color),
                                    text("interactive").size(10).color(tc.text_secondary),
                                ].spacing(4).align_y(iced::Alignment::Center))
                                    .on_press(Message::TogglePipelineStepInteractive(idx, s_idx))
                                    .style(button::text).padding([2, 4]),
                                "Interactive: launches full claude TUI; otherwise headless `claude -p`."
                            ),
                            tip(
                                button(text(match step.backend.as_deref() {
                                    None => "⚙ global".to_string(),
                                    Some(s) => AgentBackend::from_str(s).label().to_string(),
                                }).size(9).color(tc.blue))
                                    .on_press(Message::CyclePipelineStepBackend(idx, s_idx))
                                    .style(button::text).padding([2, 4]),
                                "Model for this step (click to cycle; global = settings backend)"
                            ),
                            button(text("✕").size(10).color(tc.text_muted))
                                .on_press(Message::RemovePipelineStep(idx, s_idx))
                                .style(button::text).padding([2, 6]),
                        ].spacing(6).align_y(iced::Alignment::Center)
                    );
                }
                steps_col = steps_col.push(
                    button(text("+ add step").size(10).color(tc.blue))
                        .on_press(Message::AddPipelineStep(idx))
                        .style(button::text).padding([2, 4])
                );
                card = card.push(steps_col);
            }
            let card_tc = tc.clone();
            pl_section = pl_section.push(
                container(card).padding(8)
                    .style(move |_: &Theme| styled_card(&card_tc))
            );
        }
        pl_section = pl_section.push(
            button(text("+ new pipeline").size(11).color(tc.blue))
                .on_press(Message::AddPipeline)
                .style(button::text).padding([4, 8])
        );

        container(column![
            header, rule::horizontal(1),
            scrollable(column![
                agent_section,
                Space::new().height(16),
                notion_section,
                Space::new().height(16),
                text("Color Theme").size(12).color(tc.text_muted),
                themes,
                Space::new().height(16),
                qp_section,
                Space::new().height(16),
                pl_section,
                Space::new().height(16),
                options_section,
            ].spacing(8).padding([16, 20])).height(Fill),
            rule::horizontal(1),
            footer,
        ]).center_x(Fill).height(Fill).into()
    }

    fn view_confirm_delete(&self, pi: usize, files: &[String]) -> Element<'_, Message> {
        let tc = self.tc();
        let dir_name = self.projects.get(pi)
            .map(|p| p.path.display().to_string())
            .unwrap_or_default();

        let mut content = Column::new().spacing(12).padding(24).max_width(600);
        content = content.push(text("Move Directory to Trash?").size(18).color(tc.text_primary));
        content = content.push(text(dir_name).size(12).font(MONO_FONT).color(tc.text_secondary));
        content = content.push(text("Will be moved to macOS Trash — recoverable from there.").size(11).color(tc.text_muted));

        if files.is_empty() {
            content = content.push(text("No uncommitted changes.").size(12).color(tc.text_muted));
        } else {
            content = content.push(
                text(format!("{} uncommitted files:", files.len())).size(12).color(tc.orange)
            );
            let mut file_list = Column::new().spacing(2);
            for (i, f) in files.iter().take(20).enumerate() {
                file_list = file_list.push(text(f.clone()).size(11).font(MONO_FONT).color(tc.orange));
            }
            if files.len() > 20 {
                file_list = file_list.push(text(format!("... and {} more", files.len() - 20)).size(11).color(tc.text_muted));
            }
            content = content.push(scrollable(file_list).height(200));
        }

        content = content.push(
            row![
                button(text("Cancel").size(13).color(tc.text_primary))
                    .on_press(Message::CancelDelete)
                    .style(button::secondary)
                    .padding([8, 20]),
                Space::new().width(12),
                button(text("Move to Trash").size(13).color(tc.red))
                    .on_press(Message::ConfirmDeleteDir(pi))
                    .style(button::secondary)
                    .padding([8, 20]),
            ]
        );

        container(content).center(Fill).height(Fill).into()
    }

    fn view_pipeline_modal(&self, pi: usize, si: usize) -> Element<'_, Message> {
        let tc = self.tc();
        let session_name = self.projects.get(pi)
            .and_then(|p| p.sessions.get(si))
            .map(|s| s.name.clone())
            .unwrap_or_default();

        let mut content = Column::new().spacing(14).padding(24).max_width(640);
        content = content.push(text("Run Pipeline").size(18).color(tc.text_primary));
        content = content.push(text(format!("Session: {}", session_name)).size(11).color(tc.text_muted));

        // Pipeline picker
        if self.pipelines.is_empty() {
            content = content.push(
                text("No pipelines defined yet — open Settings → Pipelines to add one.")
                    .size(12).color(tc.orange)
            );
        } else {
            content = content.push(text("Pipeline").size(11).color(tc.text_muted));
            let mut list = Column::new().spacing(4);
            for (idx, p) in self.pipelines.iter().enumerate() {
                let selected = self.pipeline_modal_selected == Some(idx);
                let icon = if selected { "◉" } else { "○" };
                let icon_color = if selected { tc.blue } else { tc.text_muted };
                list = list.push(
                    button(row![
                        text(icon).size(14).color(icon_color),
                        text(p.name.clone()).size(12).color(tc.text_primary),
                        Space::new().width(Fill),
                        text(format!("{} step{}", p.steps.len(), if p.steps.len() == 1 { "" } else { "s" }))
                            .size(10).color(tc.text_muted),
                    ].spacing(8).align_y(iced::Alignment::Center))
                    .on_press(Message::SelectPipelineForRun(idx))
                    .style(if selected { button::secondary } else { button::text })
                    .padding([6, 10]).width(Fill)
                );
            }
            content = content.push(list);
        }

        // Requirements (multi-line; Enter inserts a newline, click Start to submit)
        content = content.push(Space::new().height(8));
        content = content.push(text("Requirements (saved to .orchpipeline)").size(11).color(tc.text_muted));
        content = content.push(
            container(
                text_editor(&self.pipeline_modal_requirements)
                    .placeholder("Describe what the pipeline should accomplish... (Enter for newline)")
                    .on_action(Message::PipelineRequirementsAction)
                    .size(12)
                    .padding(8)
                    .height(220)
            )
        );
        content = content.push(
            text(".orchpipeline lives at <project>/.orchpipeline. Each step receives a pointer to it as the first line of its prompt.")
                .size(10).color(tc.text_muted)
        );

        // Buttons
        let can_start = self.pipeline_modal_selected.is_some()
            && self.pipelines.get(self.pipeline_modal_selected.unwrap_or(0))
                .map(|p| !p.steps.is_empty())
                .unwrap_or(false);
        let mut start_btn = button(text("Start").size(13).color(if can_start { tc.green } else { tc.text_muted }))
            .style(button::secondary)
            .padding([8, 20]);
        if can_start {
            start_btn = start_btn.on_press(Message::StartPipeline);
        }
        content = content.push(
            row![
                button(text("Cancel").size(13).color(tc.text_primary))
                    .on_press(Message::ClosePipelineModal)
                    .style(button::secondary)
                    .padding([8, 20]),
                Space::new().width(12),
                start_btn,
            ]
        );
        let _ = pi; let _ = si;
        container(content).center(Fill).height(Fill).into()
    }

    fn theme(&self) -> Theme { Theme::Dark }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            iced::time::every(Duration::from_secs(10)).map(|_| Message::Tick),
            iced::time::every(Duration::from_millis(1500)).map(|_| Message::Blink),
            iced::time::every(Duration::from_millis(100)).map(|_| Message::FrameProbe),
        ];
        // Subscribe to every terminal, not just the active one.
        //
        // Why: the terminal's subscription is the only consumer of the bounded
        // mpsc channel that alacritty's PTY reader thread writes into. If we
        // skip the subscription for a background terminal, its claude session
        // keeps producing output, the channel fills up, the PTY reader blocks
        // in `block_on` — and the next time anything (including a KillSession
        // from the UI) tries to send into that channel, the main thread
        // deadlocks on the channel's mutex. Full UI freeze.
        //
        // The CPU/GPU win from "subscribe only active" (v0.1.6) is not worth
        // the freeze, so draining every terminal is mandatory. Background
        // output will trigger redraws, but for on-screen widgets only the
        // active terminal is rendered, so GPU cost stays bounded.
        for (_, ti) in &self.terminals {
            subs.push(ti.terminal.subscription().map(Message::TermEvent));
        }
        Subscription::batch(subs)
    }
}

// ─── Styled helpers ───

fn styled_panel(tc: &TC) -> container::Style {
    container::Style {
        background: Some(Background::Color(tc.bg_panel)),
        border: Border { color: tc.border, width: 1.0, ..Default::default() },
        ..Default::default()
    }
}

fn styled_card(tc: &TC) -> container::Style {
    container::Style {
        background: Some(Background::Color(tc.bg_card)),
        border: Border { color: tc.border, width: 1.0, radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Convert git_info sub_repos to session SubRepoView
fn to_sub_repo_views(info: &git_info::GitInfo) -> Vec<session::SubRepoView> {
    info.sub_repos.iter().map(|r| {
        let (has_unmerged, pr_num) = match &r.pr {
            git_info::PrStatus::Open(n) => (true, n.clone()),
            git_info::PrStatus::Merged(n) => (false, n.clone()),
            git_info::PrStatus::None => (false, String::new()),
        };
        let pr_url = if !pr_num.is_empty() && !r.repo_url.is_empty() {
            let num = pr_num.trim_start_matches('#');
            format!("{}/pull/{}", r.repo_url, num)
        } else {
            String::new()
        };
        session::SubRepoView {
            name: r.name.clone(),
            branch: r.branch.clone(),
            commit_short: r.commit_short.clone(),
            dirty_files: r.dirty_files,
            unpushed_commits: r.unpushed_commits,
            has_unmerged_pr: has_unmerged,
            pr_number: pr_num,
            pr_url,
            deployments: Vec::new(),
            dev_servers: r.dev_servers.iter().map(|(url, running)| {
                session::DevServerInfo { url: url.clone(), running: *running }
            }).collect(),
        }
    }).collect()
}

/// Stable short label per Message variant used for perf bucketing.
/// Kept manual (vs Debug) so labels don't include payload data.
fn message_label(m: &Message) -> &'static str {
    match m {
        Message::OpenProject => "OpenProject",
        Message::ProjectPicked(_) => "ProjectPicked",
        Message::NewProject => "NewProject",
        Message::NewProjectFolderPicked(_) => "NewProjectFolderPicked",
        Message::AddSession(_) => "AddSession",
        Message::NewSessionInCurrentFolder => "NewSessionInCurrentFolder",
        Message::SessionNameSubmit(_, _) => "SessionNameSubmit",
        Message::SessionNameChanged(_) => "SessionNameChanged",
        Message::CancelSessionDialog => "CancelSessionDialog",
        Message::ToggleLaunchClaude => "ToggleLaunchClaude",
        Message::SelectSession(_, _) => "SelectSession",
        Message::KillSession(_, _) => "KillSession",
        Message::MakeIdle(_, _) => "MakeIdle",
        Message::CardHover(_) => "CardHover",
        Message::StartRenameSession(_, _) => "StartRenameSession",
        Message::RenameSessionInput(_) => "RenameSessionInput",
        Message::RenameSessionSubmit => "RenameSessionSubmit",
        Message::SetSessionColor(_, _, _) => "SetSessionColor",
        Message::SetNewSessionColor(_) => "SetNewSessionColor",
        Message::RemoveProject(_) => "RemoveProject",
        Message::DeleteProjectDir(_) => "DeleteProjectDir",
        Message::ConfirmDeleteDir(_) => "ConfirmDeleteDir",
        Message::CancelDelete => "CancelDelete",
        Message::ToggleProjectExpand(_) => "ToggleProjectExpand",
        Message::OpenFile(_) => "OpenFile",
        Message::RefreshProject(_) => "RefreshProject",
        Message::FetchGitInfo(_) => "FetchGitInfo",
        Message::GitInfoFetched(_, _) => "GitInfoFetched",
        Message::FetchDeployments(_) => "FetchDeployments",
        Message::DeploymentsFetched(_, _) => "DeploymentsFetched",
        Message::ToggleDeploymentDropdown => "ToggleDeploymentDropdown",
        Message::OpenUrl(_) => "OpenUrl",
        Message::VoiceToggle => "VoiceToggle",
        Message::VoiceResult(_) => "VoiceResult",
        Message::GroqKeyChanged(_) => "GroqKeyChanged",
        Message::ToggleDangerouslySkipPermissions => "ToggleDangerouslySkipPermissions",
        Message::ToggleDatePrefix => "ToggleDatePrefix",
        Message::SendQuickPrompt(_) => "SendQuickPrompt",
        Message::AddQuickPrompt => "AddQuickPrompt",
        Message::RemoveQuickPrompt(_) => "RemoveQuickPrompt",
        Message::QuickPromptInput(_) => "QuickPromptInput",
        Message::UpdateCheckResult(_) => "UpdateCheckResult",
        Message::DismissUpdate => "DismissUpdate",
        Message::TerminalExited(_, _) => "TerminalExited",
        Message::ProjectTerminalsExited(_) => "ProjectTerminalsExited",
        Message::ProjectDirTrashed(_, _) => "ProjectDirTrashed",
        Message::ResizeSidebar(_) => "ResizeSidebar",
        Message::ToggleFileExpand(_) => "ToggleFileExpand",
        Message::RefreshExplorer => "RefreshExplorer",
        Message::RefreshAll => "RefreshAll",
        Message::Tick => "Tick",
        Message::Blink => "Blink",
        Message::FrameProbe => "FrameProbe",
        Message::KeyboardEvent(_) => "KeyboardEvent",
        Message::ToggleSettings => "ToggleSettings",
        Message::SetTheme(_) => "SetTheme",
        Message::TermEvent(_) => "TermEvent",
        Message::SetAgentBackend(_) => "SetAgentBackend",
        Message::SetScreen(_) => "SetScreen",
        Message::NotionTokenChanged(_) => "NotionTokenChanged",
        Message::NotionConnect => "NotionConnect",
        Message::NotionDatabasesFetched(_) => "NotionDatabasesFetched",
        Message::NotionSelectDatabase(_) => "NotionSelectDatabase",
        Message::NotionSchemaFetched(_) => "NotionSchemaFetched",
        Message::NotionTasksFetched(_) => "NotionTasksFetched",
        Message::NotionRefresh => "NotionRefresh",
        Message::NotionSetGroupBy(_) => "NotionSetGroupBy",
        Message::BoardAddTaskStart(_) => "BoardAddTaskStart",
        Message::BoardAddTaskInput(_) => "BoardAddTaskInput",
        Message::BoardAddTaskSubmit => "BoardAddTaskSubmit",
        Message::BoardAddTaskCancel => "BoardAddTaskCancel",
        Message::BoardTaskCreated(_) => "BoardTaskCreated",
        Message::NotionOpenTask(_) => "NotionOpenTask",
        Message::OpenTaskBoard(_) => "OpenTaskBoard",
        Message::NotionBodyFetched(_, _) => "NotionBodyFetched",
        Message::NotionCommentsFetched(_, _) => "NotionCommentsFetched",
        Message::CommentInput(_) => "CommentInput",
        Message::PostComment => "PostComment",
        Message::CommentPosted(_) => "CommentPosted",
        Message::NotionCloseTask => "NotionCloseTask",
        Message::TaskOpenDir => "TaskOpenDir",
        Message::TaskNewDir => "TaskNewDir",
        Message::TaskDirPicked(_) => "TaskDirPicked",
        Message::TaskNewParentPicked(_) => "TaskNewParentPicked",
        Message::SetNewSessionBackend(_) => "SetNewSessionBackend",
        Message::ReopenTaskLink(_) => "ReopenTaskLink",
        Message::AddPipeline => "AddPipeline",
        Message::RemovePipeline(_) => "RemovePipeline",
        Message::PipelineNameChanged(_, _) => "PipelineNameChanged",
        Message::PipelinePrependChanged(_, _) => "PipelinePrependChanged",
        Message::AddPipelineStep(_) => "AddPipelineStep",
        Message::RemovePipelineStep(_, _) => "RemovePipelineStep",
        Message::MovePipelineStep(_, _, _) => "MovePipelineStep",
        Message::PipelineStepPromptChanged(_, _, _) => "PipelineStepPromptChanged",
        Message::TogglePipelineStepInteractive(_, _) => "TogglePipelineStepInteractive",
        Message::CyclePipelineStepBackend(_, _) => "CyclePipelineStepBackend",
        Message::ToggleEditPipeline(_) => "ToggleEditPipeline",
        Message::OpenRunPipelineModal(_, _) => "OpenRunPipelineModal",
        Message::ClosePipelineModal => "ClosePipelineModal",
        Message::SelectPipelineForRun(_) => "SelectPipelineForRun",
        Message::PipelineRequirementsAction(_) => "PipelineRequirementsAction",
        Message::StartPipeline => "StartPipeline",
        Message::PipelineSpawnNext(_, _) => "PipelineSpawnNext",
        Message::PipelineSendPrompt(_, _) => "PipelineSendPrompt",
        Message::PipelineSubmitPrompt(_, _) => "PipelineSubmitPrompt",
        Message::PipelineNextStep(_, _) => "PipelineNextStep",
        Message::PipelineJumpToStep(_, _, _) => "PipelineJumpToStep",
        Message::PipelineStop(_, _) => "PipelineStop",
    }
}

/// Find claude binary path
fn which_claude() -> String {
    // Use user's login shell to resolve claude path (preserves PATH order)
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    if let Ok(out) = std::process::Command::new(&shell)
        .args(["-lc", "which claude"])
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return path;
            }
        }
    }
    // Fallback to common paths
    let candidates = [
        dirs::home_dir().map(|h| h.join(".local/bin/claude").to_string_lossy().to_string()),
        dirs::home_dir().map(|h| h.join(".claude/bin/claude").to_string_lossy().to_string()),
        Some("/usr/local/bin/claude".to_string()),
        Some("/opt/homebrew/bin/claude".to_string()),
    ];
    for candidate in &candidates {
        if let Some(path) = candidate {
            if std::path::Path::new(path).exists() {
                return path.clone();
            }
        }
    }
    // Last resort
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "claude".to_string())
}

/// Prepend $HOME/.local/bin to a PATH string if not already present.
fn prepend_local_bin(path: String) -> String {
    let Some(home) = dirs::home_dir() else { return path; };
    let local_bin = home.join(".local/bin");
    let local_bin_str = local_bin.to_string_lossy().to_string();
    let already_present = path
        .split(':')
        .map(|e| e.trim_end_matches('/'))
        .any(|e| e == local_bin_str);
    if already_present {
        path
    } else if path.is_empty() {
        local_bin_str
    } else {
        format!("{}:{}", local_bin_str, path)
    }
}

fn tip<'a>(content: impl Into<Element<'a, Message>>, hint: &str) -> Element<'a, Message> {
    iced::widget::tooltip(
        content,
        container(text(hint.to_string()).size(11).color(Color::from_rgb(0.8, 0.8, 0.85)))
            .padding([3, 8])
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.25))),
                border: Border { color: Color::from_rgb(0.3, 0.3, 0.35), width: 1.0, radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }),
        iced::widget::tooltip::Position::Bottom,
    ).into()
}

/// Check GitHub releases for newer version
async fn check_latest_version() -> Option<String> {
    let output = tokio::process::Command::new("gh")
        .args(["api", "repos/foker/orch-ide/releases/latest", "--jq", ".tag_name"])
        .output()
        .await
        .ok()?;

    if !output.status.success() { return None; }
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let latest = tag.trim_start_matches('v');
    if latest != APP_VERSION && latest > APP_VERSION {
        Some(latest.to_string())
    } else {
        None
    }
}

fn chip<'a>(label: &str, color: Color) -> Element<'a, Message> {
    let bg = Color { a: 0.1, ..color };
    container(text(label.to_string()).size(11).font(MONO_FONT).color(color))
        .padding([3, 10])
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

/// Pure decision: given backend + flags, return (program, args) for the terminal.
/// `launch_agent` false => plain login shell. Resume only meaningful for Claude.
fn build_agent_command(
    backend: AgentBackend,
    launch_agent: bool,
    resume: bool,
    claude_path: &str,
    opencode_path: &str,
    codex_path: &str,
    session_name: &str,
    skip_perms: bool,
) -> (String, Vec<String>) {
    if !launch_agent {
        let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        return (sh, vec![]);
    }
    match backend {
        // opencode has no CLI permission flag — "no restrictions" is configured
        // in ~/.config/opencode/opencode.jsonc (permission: allow). Same for mimo.
        AgentBackend::OpenCode => (opencode_path.to_string(), vec![]),
        AgentBackend::Codex => {
            let mut a = vec![];
            if skip_perms { a.push("--dangerously-bypass-approvals-and-sandbox".to_string()); }
            (codex_path.to_string(), a)
        }
        // Xiaomi MiMo is an OpenAI-compatible provider configured in opencode;
        // launch opencode pinned to the mimo model.
        AgentBackend::XiaomiMimo => (opencode_path.to_string(), vec!["-m".to_string(), MIMO_MODEL.to_string()]),
        AgentBackend::ClaudeCode => {
            let skip_flag = if skip_perms { " --dangerously-skip-permissions" } else { "" };
            if resume {
                let cmd = format!(
                    "{} --continue{} 2>/dev/null || {} --name '{}'{}",
                    claude_path, skip_flag, claude_path,
                    session_name.replace('\'', "'\\''"), skip_flag
                );
                ("/bin/sh".to_string(), vec!["-c".to_string(), cmd])
            } else {
                let mut a = vec!["--name".to_string(), session_name.to_string()];
                if skip_perms { a.push("--dangerously-skip-permissions".to_string()); }
                (claude_path.to_string(), a)
            }
        }
    }
}

/// Find opencode binary path (mirrors which_claude).
fn which_opencode() -> String {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    if let Ok(out) = std::process::Command::new(&shell)
        .args(["-lc", "which opencode"]).output()
    {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() { return p; }
        }
    }
    let fallbacks = [
        dirs::home_dir().map(|h| h.join(".opencode/bin/opencode").to_string_lossy().to_string()),
        Some("/usr/local/bin/opencode".to_string()),
        Some("/opt/homebrew/bin/opencode".to_string()),
    ];
    for f in fallbacks.into_iter().flatten() {
        if std::path::Path::new(&f).exists() { return f; }
    }
    "opencode".to_string()
}

/// Find codex binary path (mirrors which_claude).
fn which_codex() -> String {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    if let Ok(out) = std::process::Command::new(&shell)
        .args(["-lc", "which codex"]).output()
    {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() { return p; }
        }
    }
    let fallbacks = [
        Some("/opt/homebrew/bin/codex".to_string()),
        Some("/usr/local/bin/codex".to_string()),
        dirs::home_dir().map(|h| h.join(".codex/bin/codex").to_string_lossy().to_string()),
    ];
    for f in fallbacks.into_iter().flatten() {
        if std::path::Path::new(&f).exists() { return f; }
    }
    "codex".to_string()
}

/// Sanitize a task title into a safe directory name (kebab-case).
fn sanitize_dir_name(title: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in title.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() { "task".to_string() } else { trimmed }
}

/// Render a Notion task to markdown for the TASK-*.md seed file.
fn task_to_markdown(task: &notion::NotionTask, schema: &[notion::NotionProp]) -> String {
    use notion::PropValue::*;
    let mut s = format!("# {}\n\n", if task.title.is_empty() { "Untitled task" } else { &task.title });
    if !task.url.is_empty() { s.push_str(&format!("Notion: {}\n\n", task.url)); }
    s.push_str("## Properties\n\n");
    for p in schema {
        if matches!(p.kind, notion::PropKind::Title) { continue; }
        let v = match task.props.get(&p.id) {
            Some(Text(t)) if !t.is_empty() => t.clone(),
            Some(Number(n)) => n.to_string(),
            Some(Checkbox(b)) => if *b {"yes".into()} else {"no".into()},
            Some(Url(u)) if !u.is_empty() => u.clone(),
            Some(Date(d)) => d.clone(),
            Some(Select(Some(o))) => o.name.clone(),
            Some(MultiSelect(v)) if !v.is_empty() => v.iter().map(|o| o.name.clone()).collect::<Vec<_>>().join(", "),
            Some(People(v)) if !v.is_empty() => v.join(", "),
            _ => continue,
        };
        s.push_str(&format!("- {}: {}\n", p.name, v));
    }
    s
}

/// Group tasks into ordered columns by the group-by property's options,
/// plus a trailing "(none)" column. Returns Vec<(column_label, task_refs)>.
fn group_tasks<'a>(
    tasks: &'a [notion::NotionTask],
    group_prop_id: &str,
    options: &[notion::SelectOption],
) -> Vec<(String, Vec<&'a notion::NotionTask>)> {
    let mut cols: Vec<(String, Vec<&notion::NotionTask>)> =
        options.iter().map(|o| (o.name.clone(), Vec::new())).collect();
    let mut none_bucket: Vec<&notion::NotionTask> = Vec::new();
    for t in tasks {
        if t.title.trim().is_empty() { continue; } // hide titleless tasks
        let name = match t.props.get(group_prop_id) {
            Some(notion::PropValue::Select(Some(o))) => Some(o.name.clone()),
            _ => None,
        };
        match name {
            Some(n) => {
                if let Some(col) = cols.iter_mut().find(|(label, _)| *label == n) { col.1.push(t); }
                else { none_bucket.push(t); }
            }
            None => none_bucket.push(t),
        }
    }
    cols.push(("(none)".to_string(), none_bucket));
    cols
}

#[cfg(test)]
mod board_md_tests {
    use super::*;
    use notion::*;
    use std::collections::HashMap;

    fn task(id: &str, status: Option<&str>) -> NotionTask {
        let mut props = HashMap::new();
        props.insert("stat".to_string(), match status {
            Some(s) => PropValue::Select(Some(SelectOption{id:s.into(),name:s.into(),color:"gray".into()})),
            None => PropValue::Empty,
        });
        NotionTask { id: id.into(), title: id.into(), url: "".into(), props }
    }

    #[test]
    fn groups_tasks_by_status_with_none_bucket() {
        let opts = vec![
            SelectOption{id:"Todo".into(),name:"Todo".into(),color:"gray".into()},
            SelectOption{id:"Done".into(),name:"Done".into(),color:"green".into()},
        ];
        let tasks = vec![task("a", Some("Todo")), task("b", Some("Done")), task("c", None)];
        let cols = group_tasks(&tasks, "stat", &opts);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].0, "Todo"); assert_eq!(cols[0].1.len(), 1);
        assert_eq!(cols[2].0, "(none)"); assert_eq!(cols[2].1.len(), 1);
    }

    #[test]
    fn renders_title_url_and_a_prop() {
        let mut props = HashMap::new();
        props.insert("stat".into(), PropValue::Select(Some(SelectOption{id:"s".into(),name:"Todo".into(),color:"gray".into()})));
        let t = NotionTask { id:"pg1".into(), title:"Fix login".into(), url:"https://notion.so/pg1".into(), props };
        let schema = vec![
            NotionProp{id:"title".into(),name:"Name".into(),kind:PropKind::Title},
            NotionProp{id:"stat".into(),name:"Status".into(),kind:PropKind::Status(vec![])},
        ];
        let md = task_to_markdown(&t, &schema);
        assert!(md.starts_with("# Fix login"));
        assert!(md.contains("https://notion.so/pg1"));
        assert!(md.contains("Status: Todo"));
    }
}

#[cfg(test)]
mod agent_cmd_tests {
    use super::*;

    #[test]
    fn claude_fresh_uses_name_flag() {
        let (prog, args) = build_agent_command(
            AgentBackend::ClaudeCode, true, false,
            "/bin/claude", "/bin/opencode", "/bin/codex", "my task", true,
        );
        assert_eq!(prog, "/bin/claude");
        assert_eq!(args, vec!["--name", "my task", "--dangerously-skip-permissions"]);
    }

    #[test]
    fn claude_resume_uses_continue_shell() {
        let (prog, args) = build_agent_command(
            AgentBackend::ClaudeCode, true, true,
            "/bin/claude", "/bin/opencode", "/bin/codex", "my task", false,
        );
        assert_eq!(prog, "/bin/sh");
        assert_eq!(args[0], "-c");
        assert!(args[1].contains("--continue"));
    }

    #[test]
    fn opencode_runs_bare_in_cwd() {
        let (prog, args) = build_agent_command(
            AgentBackend::OpenCode, true, false,
            "/bin/claude", "/bin/opencode", "/bin/codex", "my task", true,
        );
        assert_eq!(prog, "/bin/opencode");
        assert!(args.is_empty());
    }

    #[test]
    fn codex_bypass_when_skip_perms() {
        let (prog, args) = build_agent_command(
            AgentBackend::Codex, true, false,
            "/bin/claude", "/bin/opencode", "/bin/codex", "my task", true,
        );
        assert_eq!(prog, "/bin/codex");
        assert_eq!(args, vec!["--dangerously-bypass-approvals-and-sandbox"]);
    }

    #[test]
    fn codex_bare_when_no_skip() {
        let (prog, args) = build_agent_command(
            AgentBackend::Codex, true, false,
            "/bin/claude", "/bin/opencode", "/bin/codex", "my task", false,
        );
        assert_eq!(prog, "/bin/codex");
        assert!(args.is_empty());
    }

    #[test]
    fn mimo_runs_opencode_with_model() {
        let (prog, args) = build_agent_command(
            AgentBackend::XiaomiMimo, true, false,
            "/bin/claude", "/bin/opencode", "/bin/codex", "my task", true,
        );
        assert_eq!(prog, "/bin/opencode");
        assert_eq!(args, vec!["-m", "mimo/mimo-v2.5-pro"]);
    }

    #[test]
    fn plain_shell_when_agent_off() {
        let (prog, _args) = build_agent_command(
            AgentBackend::ClaudeCode, false, false,
            "/bin/claude", "/bin/opencode", "/bin/codex", "x", true,
        );
        assert!(prog.ends_with("sh") || prog.ends_with("zsh") || prog.contains("/"));
    }
}
