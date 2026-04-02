mod session;
mod explorer;
mod git_info;
mod hooks;
mod persistence;
mod logging;
mod voice;

use iced::widget::{
    button, column, container, row, scrollable, text,
    text_input, Column, Row, Space, rule,
};
use iced::{Element, Fill, Font, Padding, Subscription, Theme, Color, Border, Background, Task};
use session::{ProjectGroup, Session, SessionStatus};
use std::path::PathBuf;
use std::time::Duration;

const MONO_FONT: Font = Font::with_name("JetBrains Mono");
const SESSION_INPUT_ID: &str = "session-name-input";
const APP_VERSION: &str = "0.1.4";

fn main() -> iced::Result {
    logging::init();
    iced::application(App::boot, App::update, App::view)
        .title("Claude Sessions")
        .theme(App::theme)
        .subscription(App::subscription)
        .window_size((1200.0, 800.0))
        .run()
}

// ─── Theme System ───

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppTheme { Midnight, VsCode, Darcula, GitHub, Monokai, Catppuccin }

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
    AddSession(usize), SessionNameSubmit(usize, String), SessionNameChanged(String),
    ToggleLaunchClaude,
    SelectSession(usize, usize), KillSession(usize, usize), MakeIdle(usize, usize),
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
    // Quick prompts
    SendQuickPrompt(String),
    AddQuickPrompt, RemoveQuickPrompt(usize), QuickPromptInput(String),
    UpdateCheckResult(Option<String>), // Some("0.2.0") if newer version available
    DismissUpdate,
    ResizeSidebar(f32),
    ToggleFileExpand(usize), RefreshExplorer, RefreshAll, Tick, Blink,
    KeyboardEvent(iced::keyboard::Event),
    ToggleSettings, SetTheme(AppTheme),
    TermEvent(iced_term::Event),
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
    launch_claude: bool,
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
    tick_count: u32,
    blink_on: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            projects: Vec::new(), active_project: None, active_session: None,
            file_entries: Vec::new(), terminals: Vec::new(), next_term_id: 0,
            session_name_input: String::new(), show_session_dialog: None, new_project_parent: None,
            launch_claude: true,
            show_settings: false, confirm_delete: None, renaming_session: None, rename_input: String::new(), new_session_color: session::SessionColor::Grey,
            show_deployment_dropdown: false,
            dangerously_skip_permissions: true, quick_prompts: Vec::new(), quick_prompt_input: String::new(), update_available: None,
            voice_recorder: voice::AudioRecorder::new(), groq_api_key: String::new(), voice_transcribing: false,
            current_theme: AppTheme::Midnight, sidebar_width: 280.0, tick_count: 0, blink_on: true,
        }
    }
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let mut app = Self::default();
        // Load persisted state
        if let Some(state) = persistence::load() {
            app.projects = state.projects;
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
        (app, check_update)
    }
    fn tc(&self) -> TC { self.current_theme.colors() }

    fn spawn_session_terminal(&mut self, pi: usize, si: usize, resume: bool) {
        app_log!("spawn_terminal: pi={} si={} resume={} (active terminals: {})", pi, si, resume, self.terminals.len());
        let cwd = self.projects[pi].path.clone();
        let session_name = self.projects[pi].sessions[si].name.clone();
        let tid = self.next_term_id;
        self.next_term_id += 1;

        // Determine program and args
        let skip_flag = if self.dangerously_skip_permissions { " --dangerously-skip-permissions" } else { "" };
        let (program, args) = if self.launch_claude {
            let claude_path = which_claude();
            if resume {
                let cmd = format!(
                    "{} --continue{} 2>/dev/null || {} --name '{}'{}",
                    claude_path, skip_flag, claude_path, session_name.replace('\'', "'\\''"), skip_flag
                );
                ("/bin/sh".to_string(), vec!["-c".to_string(), cmd])
            } else {
                let mut a = vec!["--name".to_string(), session_name];
                if self.dangerously_skip_permissions { a.push("--dangerously-skip-permissions".to_string()); }
                (claude_path, a)
            }
        } else {
            (std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()), vec![])
        };

        let mut env = std::collections::HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        // Inherit PATH from user shell
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        } else {
            env.insert("PATH".to_string(), format!("{}/.local/bin:/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin",
                dirs::home_dir().unwrap_or_default().display()));
        }

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
            // Setup hooks
            let sid = &self.projects[pi].sessions[si].id;
            if let Ok(hp) = hooks::create_hook_script(sid) {
                let _ = hooks::configure_claude_hooks(&self.projects[pi].path, &hp);
            }
            self.terminals.push(((pi, si), TerminalInstance { terminal, id: tid }));
        }
    }

    fn save_state(&self) {
        persistence::save(&persistence::AppState {
            projects: self.projects.clone(),
            theme: format!("{:?}", self.current_theme),
            sidebar_width: self.sidebar_width,
            groq_api_key: self.groq_api_key.clone(),
            dangerously_skip_permissions: self.dangerously_skip_permissions,
            quick_prompts: self.quick_prompts.clone(),
        });
    }

    fn update(&mut self, message: Message) -> Task<Message> {
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
                        self.launch_claude = true;
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
                    self.launch_claude = true;
                }
                iced::widget::operation::focus_next()
            }
            Message::ToggleLaunchClaude => { self.launch_claude = !self.launch_claude; Task::none() }
            Message::AddSession(pi) => {
                self.show_session_dialog = Some(pi);
                iced::widget::operation::focus_next()
            }
            Message::SessionNameChanged(n) => { self.session_name_input = n; Task::none() }
            Message::SessionNameSubmit(pi, name) => {
                app_log!("SessionNameSubmit: pi={} name={}", pi, name);
                if !name.is_empty() {
                    if pi == usize::MAX {
                        // "+" flow: create new folder in parent
                        if let Some(parent) = self.new_project_parent.take() {
                            let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                            let folder_name = format!("{}-{}", date, name);
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
            Message::SelectSession(pi, si) => {
                app_log!("SelectSession: pi={} si={}", pi, si);
                self.active_session = Some((pi, si));
                self.active_project = Some(pi);
                self.file_entries = explorer::read_directory(&self.projects[pi].path, 0);
                self.show_deployment_dropdown = false;
                // Spawn terminal if it doesn't exist (e.g. after restart)
                let has_term = self.terminals.iter().any(|(k, _)| *k == (pi, si));
                if !has_term && pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.spawn_session_terminal(pi, si, true);
                }
                // Fetch git info + deployments async (non-blocking)
                let git_task = self.update(Message::FetchGitInfo(pi));
                let dep_task = self.update(Message::FetchDeployments(pi));
                return Task::batch([git_task, dep_task]);
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
                // Remove terminal
                self.terminals.retain(|(k, _)| *k != (pi, si));
                // Remove session
                if pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.projects[pi].sessions.remove(si);
                }
                // Fix active session reference
                if self.active_session == Some((pi, si)) {
                    self.active_session = None;
                }
                // Fix terminal indices after removal
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
                    let path = self.projects[pi].path.clone();
                    app_log!("DELETING directory: {:?}", path);
                    if let Err(e) = std::fs::remove_dir_all(&path) {
                        app_log!("Failed to delete: {}", e);
                    }
                    // Also remove from projects list
                    self.confirm_delete = None;
                    return self.update(Message::RemoveProject(pi));
                }
                self.confirm_delete = None;
                Task::none()
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
                let path = self.projects[pi].path.clone();
                Task::perform(
                    async move { git_info::get_git_info_with_pr(&path) },
                    move |info| Message::GitInfoFetched(pi, info),
                )
            }
            Message::GitInfoFetched(pi, info) => {
                app_log!("GitInfoFetched: pi={} has_info={}", pi, info.is_some());
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
                    }
                }
                Task::none()
            }
            Message::FetchDeployments(pi) => {
                if pi >= self.projects.len() { return Task::none(); }
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
                        let mut results = Vec::new();
                        for (name, path) in sub_repos {
                            let deps = git_info::get_deployments(&path);
                            if !deps.is_empty() {
                                results.push((name, deps));
                            }
                        }
                        (pi, results)
                    },
                    |(pi, deps)| Message::DeploymentsFetched(pi, deps),
                )
            }
            Message::DeploymentsFetched(pi, deps) => {
                app_log!("DeploymentsFetched: pi={} total={}", pi, deps.iter().map(|(_, d)| d.len()).sum::<usize>());
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
                            _ => {}
                        }
                    }
                }
                Task::none()
            }
            Message::Tick => {
                // Only check session statuses via hooks (lightweight, no network)
                for p in &mut self.projects {
                    for s in &mut p.sessions {
                        if let Some(pay) = hooks::read_status(&s.id) {
                            if let Some(st) = pay.to_session_status() {
                                if st != s.status {
                                    s.status_changed_at = chrono::Utc::now();
                                    s.status = st;
                                }
                            }
                        }
                    }
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
                        if let Some((_, ti)) = self.terminals.iter_mut().find(|(_, t)| t.id == id) {
                            ti.terminal.handle(iced_term::Command::ProxyToBackend(cmd));
                        }
                    }
                }
                Task::none()
            }
        }
    }

    // ─── VIEW ───

    fn view(&self) -> Element<'_, Message> {
        let tc = self.tc();
        let center = if let Some((pi, ref files)) = self.confirm_delete {
            self.view_confirm_delete(pi, files)
        } else if self.show_settings {
            self.view_settings()
        } else {
            self.view_terminal()
        };

        let tc1 = tc.clone();
        let tc2 = tc.clone();
        let tc3 = tc.clone();
        let tc4 = tc.clone();

        let main = row![
            container(self.view_sessions()).width(self.sidebar_width).height(Fill)
                .style(move |_: &Theme| styled_panel(&tc1)),
            container(center).width(Fill).height(Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(c(0x17, 0x1b, 0x21))), ..Default::default()
                }),
            container(self.view_explorer()).width(260).height(Fill)
                .style(move |_: &Theme| styled_panel(&tc3)),
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
                tip(button(text("📁").size(12)).on_press(Message::OpenProject).style(button::text).padding(2), "Open folder (⌘O)"),
                tip(button(text("+").size(14).color(tc.text_muted)).on_press(Message::NewProject).style(button::text).padding(2), "New project (⌘N)"),
            ].spacing(4).align_y(iced::Alignment::Center),
        ).padding([10, 14]);

        let mut content = Column::new().spacing(4).padding(8);

        // New project dialog (when "+" pressed and parent folder selected)
        if self.show_session_dialog == Some(usize::MAX) {
            let check_color = if self.launch_claude { tc.green } else { tc.text_muted };
            let check_icon = if self.launch_claude { "☑" } else { "☐" };
            let parent_name = self.new_project_parent.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            content = content.push(container(column![
                text(format!("New project in: {}/", parent_name)).size(11).color(tc.text_secondary),
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
                // Pulsing dot for AWAIT status
                let dot_color = if session.status == SessionStatus::AwaitingInput && !self.blink_on {
                    Color { a: 0.3, ..sc }
                } else {
                    sc
                };
                top = top.push(text("●").size(8).color(dot_color));
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
                    // Pencil icon (visible on card hover via parent button hover state)
                    top = top.push(
                        button(text("✏").size(10).color(Color { a: 0.4, ..tc.text_muted }))
                            .on_press(Message::StartRenameSession(pi, si))
                            .style(button::text).padding([0, 2])
                    );
                    top = top.push(text(&session.name).size(13).color(tc.text_primary));
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
                        let mut r = Row::new().spacing(4);
                        r = r.push(text(label).size(9).font(MONO_FONT).color(tc.purple));
                        // PR dot: blue = unmerged, grey = none/merged
                        let pr_color = if sr.has_unmerged_pr { tc.blue } else { tc.text_muted };
                        r = r.push(text("⬤").size(6).color(pr_color));
                        if sr.dirty_files > 0 {
                            r = r.push(text(format!("· {} files", sr.dirty_files)).size(9).font(MONO_FONT).color(tc.orange));
                        }
                        col = col.push(r);
                    }
                    col.into()
                } else {
                    let mut r = Row::new().spacing(4);
                    r = r.push(text(format!("⎇ {}", project.branch)).size(10).font(MONO_FONT).color(tc.purple));
                    // PR dot
                    if let Some(sr) = project.sub_repos.first() {
                        let pr_color = if sr.has_unmerged_pr { tc.blue } else { tc.text_muted };
                        r = r.push(text("⬤").size(6).color(pr_color));
                    }
                    r = r.push(text("·").size(10).color(tc.text_muted));
                    if project.dirty_files > 0 {
                        r = r.push(text(format!("{} files", project.dirty_files)).size(10).font(MONO_FONT).color(tc.orange));
                    } else {
                        r = r.push(text("✓ clean").size(10).font(MONO_FONT).color(tc.text_muted));
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

                // Card row: [card button] [kill button]
                let card_row = row![
                    button({
                        let mut card_col = Column::new().spacing(6);
                        card_col = card_col.push(top);
                        card_col = card_col.push(meta);
                        // Color picker when renaming
                        if self.renaming_session == Some((pi, si)) {
                            let mut color_row = Row::new().spacing(4);
                            for &clr in session::SessionColor::all() {
                                let (r, g, b) = clr.to_rgb();
                                let is_selected = session.color == clr;
                                let dot_size = if is_selected { 12 } else { 10 };
                                color_row = color_row.push(
                                    button(text("●").size(dot_size).color(c(r, g, b)))
                                        .on_press(Message::SetSessionColor(pi, si, clr))
                                        .style(button::text).padding([1, 2])
                                );
                            }
                            card_col = card_col.push(color_row);
                        }
                        card_col
                    })
                        .on_press(Message::SelectSession(pi, si))
                        .style(move |_: &Theme, status: button::Status| {
                            let bg = match status {
                                button::Status::Hovered | button::Status::Pressed => hover_bg,
                                _ => bg_c,
                            };
                            button::Style {
                                background: Some(Background::Color(bg)),
                                border: Border { color: border_c, width: 1.0, radius: 6.0.into(), ..Default::default() },
                                text_color: Color::WHITE,
                                ..Default::default()
                            }
                        })
                        .padding([10, 12])
                        .width(Fill),
                    column![
                        tip(button(text("✕").size(11).color(tc.text_muted))
                                .on_press(Message::KillSession(pi, si))
                                .style(button::text).padding([4, 6]),
                            "Kill session"),
                        tip(button(text("◼").size(9).color(tc.text_muted))
                                .on_press(Message::MakeIdle(pi, si))
                                .style(button::text).padding([4, 6]),
                            "Set idle"),
                        tip(button(text("🗑").size(9))
                                .on_press(Message::DeleteProjectDir(pi))
                                .style(button::text).padding([4, 6]),
                            "Delete folder"),
                    ].spacing(0),
                ].align_y(iced::Alignment::Start);

                content = content.push(
                    container(card_row)
                        .padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 12.0 })
                );
            }

            // Add session button or inline input
            if self.show_session_dialog == Some(pi) {
                let check_color = if self.launch_claude { tc.green } else { tc.text_muted };
                let check_icon = if self.launch_claude { "☑" } else { "☐" };
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

        column![header, rule::horizontal(1), scrollable(content).height(Fill)].into()
    }

    fn view_terminal(&self) -> Element<'_, Message> {
        let tc = self.tc();

        if self.active_session.is_none() {
            return container(
                column![
                    text("◆").size(32).color(tc.text_muted),
                    text("Select or create a session").size(14).color(tc.text_muted),
                ].spacing(8).align_x(iced::Alignment::Center)
            ).center(Fill).height(Fill).into();
        }

        let (pi, si) = self.active_session.unwrap();
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
        top_row = top_row.push(
            tip(button(text("⟳").size(10).color(tc.text_muted))
                .on_press(Message::RefreshProject(pi))
                .style(button::text).padding([2, 4]),
                "Refresh git & deployments")
        );
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
                scrollable(branch_cards).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())),
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
            let short = if prompt.len() > 25 { format!("{}...", &prompt[..22]) } else { prompt.clone() };
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

        let mut main_col = Column::new();
        main_col = main_col.push(info_bar);
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

        // Groq API key section
        let groq_section = column![
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

        container(column![
            header, rule::horizontal(1),
            scrollable(column![
                text("Color Theme").size(12).color(tc.text_muted),
                themes,
                Space::new().height(16),
                qp_section,
                Space::new().height(16),
                groq_section,
            ].spacing(8).padding([16, 20])).height(Fill),
        ]).center_x(Fill).height(Fill).into()
    }

    fn view_confirm_delete(&self, pi: usize, files: &[String]) -> Element<'_, Message> {
        let tc = self.tc();
        let dir_name = self.projects.get(pi)
            .map(|p| p.path.display().to_string())
            .unwrap_or_default();

        let mut content = Column::new().spacing(12).padding(24).max_width(600);
        content = content.push(text("Delete Directory?").size(18).color(tc.text_primary));
        content = content.push(text(dir_name).size(12).font(MONO_FONT).color(tc.text_secondary));

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
                button(text("Delete").size(13).color(tc.red))
                    .on_press(Message::ConfirmDeleteDir(pi))
                    .style(button::secondary)
                    .padding([8, 20]),
            ]
        );

        container(content).center(Fill).height(Fill).into()
    }

    fn theme(&self) -> Theme { Theme::Dark }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            iced::time::every(Duration::from_secs(10)).map(|_| Message::Tick),
            iced::time::every(Duration::from_millis(1500)).map(|_| Message::Blink),
        ];
        for (_, ti) in &self.terminals { subs.push(ti.terminal.subscription().map(Message::TermEvent)); }
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
            dirty_files: r.dirty_files,
            unpushed_commits: r.unpushed_commits,
            has_unmerged_pr: has_unmerged,
            pr_number: pr_num,
            pr_url,
            deployments: Vec::new(),
        }
    }).collect()
}

/// Find claude binary path
fn which_claude() -> String {
    // Use login shell to resolve claude path (picks up nvm, homebrew, etc.)
    if let Ok(out) = std::process::Command::new("/bin/sh")
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
