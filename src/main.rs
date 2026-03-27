mod session;
mod explorer;
mod git_info;
mod hooks;
mod persistence;
mod logging;

use iced::widget::{
    button, column, container, row, scrollable, text,
    text_input, Column, Row, Space, rule,
};
use iced::{Element, Fill, Font, Padding, Subscription, Theme, Color, Border, Background, Task};
use session::{ProjectGroup, Session, SessionStatus};
use std::path::PathBuf;
use std::time::Duration;

const MONO_FONT: Font = Font::with_name("JetBrains Mono");

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
    SelectSession(usize, usize), KillSession(usize, usize), RemoveProject(usize), ToggleProjectExpand(usize),
    ResizeSidebar(f32),
    ToggleFileExpand(usize), RefreshExplorer, RefreshAll, Tick,
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
    current_theme: AppTheme,
    sidebar_width: f32,
    tick_count: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            projects: Vec::new(), active_project: None, active_session: None,
            file_entries: Vec::new(), terminals: Vec::new(), next_term_id: 0,
            session_name_input: String::new(), show_session_dialog: None, new_project_parent: None,
            launch_claude: true,
            show_settings: false, current_theme: AppTheme::Midnight, sidebar_width: 280.0, tick_count: 0,
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
            // Refresh git info
            for p in &mut app.projects {
                if let Some(info) = git_info::get_git_info_with_pr(&p.path) {
                    p.branch = info.branch.clone(); p.dirty_files = info.dirty_files;
                    p.sub_repos = to_sub_repo_views(&info);
                }
            }
        }
        (app, Task::none())
    }
    fn tc(&self) -> TC { self.current_theme.colors() }

    fn spawn_session_terminal(&mut self, pi: usize, si: usize, resume: bool) {
        app_log!("spawn_terminal: pi={} si={} resume={}", pi, si, resume);
        let cwd = self.projects[pi].path.clone();
        let session_name = self.projects[pi].sessions[si].name.clone();
        let tid = self.next_term_id;
        self.next_term_id += 1;

        // Determine program and args
        let (program, args) = if self.launch_claude {
            let claude_path = which_claude();
            if resume {
                // Try --continue, fallback to --name if no conversation found
                let cmd = format!(
                    "{} --continue --dangerously-skip-permissions 2>/dev/null || {} --name '{}' --dangerously-skip-permissions",
                    claude_path, claude_path, session_name.replace('\'', "'\\''")
                );
                ("/bin/sh".to_string(), vec!["-c".to_string(), cmd])
            } else {
                (claude_path, vec!["--name".to_string(), session_name, "--dangerously-skip-permissions".to_string()])
            }
        } else {
            (std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()), vec![])
        };

        let settings = iced_term::settings::Settings {
            backend: iced_term::settings::BackendSettings {
                program,
                args,
                working_directory: Some(cwd),
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
                Task::none()
            }
            Message::NewProjectFolderPicked(path) => {
                if let Some(parent) = path {
                    self.new_project_parent = Some(parent);
                    // Show session name dialog (no project created yet — will create on submit)
                    self.session_name_input = String::new();
                    self.show_session_dialog = Some(usize::MAX); // sentinel: means "new project"
                    self.launch_claude = true;
                }
                Task::none()
            }
            Message::ToggleLaunchClaude => { self.launch_claude = !self.launch_claude; Task::none() }
            Message::AddSession(pi) => { self.show_session_dialog = Some(pi); Task::none() }
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
                                let session = Session::new(name);
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
                        let session = Session::new(name);
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
                // Spawn terminal if it doesn't exist (e.g. after restart)
                let has_term = self.terminals.iter().any(|(k, _)| *k == (pi, si));
                if !has_term && pi < self.projects.len() && si < self.projects[pi].sessions.len() {
                    self.spawn_session_terminal(pi, si, true);
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
        let center = if self.show_settings { self.view_settings() } else { self.view_terminal() };

        let tc1 = tc.clone();
        let tc2 = tc.clone();
        let tc3 = tc.clone();
        let tc4 = tc.clone();

        let main = row![
            container(self.view_sessions()).width(self.sidebar_width).height(Fill)
                .style(move |_: &Theme| styled_panel(&tc1)),
            container(center).width(Fill).height(Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(tc2.bg_terminal)), ..Default::default()
                }),
            container(self.view_explorer()).width(260).height(Fill)
                .style(move |_: &Theme| styled_panel(&tc3)),
        ];

        let status = container(
            row![
                text(format!("● {} sessions", self.projects.iter().map(|p| p.sessions.len()).sum::<usize>()))
                    .size(11).color(tc.text_muted),
                Space::new().width(Fill),
                text("Claude Sessions v0.1.0").size(11).color(tc.text_muted),
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
                button(text("◂").size(9).color(tc.text_muted)).on_press(Message::ResizeSidebar(-40.0)).style(button::text).padding(2),
                button(text("▸").size(9).color(tc.text_muted)).on_press(Message::ResizeSidebar(40.0)).style(button::text).padding(2),
                button(text("⟳").size(13).color(tc.text_muted)).on_press(Message::RefreshAll).style(button::text).padding(2),
                button(text("◂").size(9).color(tc.text_muted)).on_press(Message::ResizeSidebar(-40.0)).style(button::text).padding(2),
                button(text("▸").size(9).color(tc.text_muted)).on_press(Message::ResizeSidebar(40.0)).style(button::text).padding(2),
                button(text("⚙").size(13).color(tc.text_muted)).on_press(Message::ToggleSettings).style(button::text).padding(2),
                button(text("📁").size(12)).on_press(Message::OpenProject).style(button::text).padding(2),
                button(text("+").size(14).color(tc.text_muted)).on_press(Message::NewProject).style(button::text).padding(2),
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
                    .on_input(Message::SessionNameChanged)
                    .on_submit(Message::SessionNameSubmit(usize::MAX, self.session_name_input.clone()))
                    .size(12).padding(6),
                button(
                    row![
                        text(check_icon).size(14).color(check_color),
                        text("Launch Claude").size(11).color(tc.text_secondary),
                    ].spacing(6).align_y(iced::Alignment::Center)
                ).on_press(Message::ToggleLaunchClaude).style(button::text).padding([2, 0]),
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
            // Project header with remove button
            content = content.push(
                row![
                    button(row![
                        text("▾").size(11).color(tc.text_muted),
                        text(&project.name).size(11).color(tc.text_secondary),
                    ].spacing(6))
                    .on_press(Message::ToggleProjectExpand(pi)).style(button::text).padding([4, 8])
                    .width(Fill),
                    button(text("✕").size(10).color(tc.text_muted))
                        .on_press(Message::RemoveProject(pi))
                        .style(button::text)
                        .padding([4, 6]),
                ].align_y(iced::Alignment::Center)
            );

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
                top = top.push(text("●").size(8).color(sc));
                top = top.push(text(&session.name).size(13).color(tc.text_primary));
                if session.background_agents > 0 {
                    top = top.push(text(format!("🤖{}", session.background_agents)).size(10).color(tc.green));
                }
                top = top.push(Space::new().width(Fill));
                // Status badge
                let status_bg = Color { a: 0.15, ..sc };
                top = top.push(
                    container(text(session.status.to_string()).size(9).color(sc))
                        .padding([2, 6])
                        .style(move |_: &Theme| container::Style {
                            background: Some(Background::Color(status_bg)),
                            border: Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        })
                );

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
                        r = r.push(text("✓ clean").size(10).font(MONO_FONT).color(tc.green));
                    }
                    r.into()
                };

                let card_content = column![top, meta].spacing(6);

                let border_c = if is_active { t.border_active } else { t.border };
                let bg_c = if is_active { t.bg_card_hover } else { t.bg_card };
                let card_container = container(card_content)
                    .padding([10, 12])
                    .width(Fill)
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(bg_c)),
                        border: Border { color: border_c, width: 1.0, radius: 6.0.into(), ..Default::default() },
                        ..Default::default()
                    });

                // Card row: [card button] [kill button]
                let card_row = row![
                    button(card_container)
                        .on_press(Message::SelectSession(pi, si))
                        .style(button::text)
                        .padding(0)
                        .width(Fill),
                    button(text("✕").size(11).color(tc.text_muted))
                        .on_press(Message::KillSession(pi, si))
                        .style(button::text)
                        .padding([8, 6]),
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
            } else {
                content = content.push(
                    button(text("+ add session").size(10).color(tc.text_muted))
                        .on_press(Message::AddSession(pi)).style(button::text).padding([4, 20])
                );
            }
            content = content.push(rule::horizontal(1));
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
        let mut info_chips = Row::new().spacing(8).align_y(iced::Alignment::Center);
        info_chips = info_chips.push(chip(status_text, sc));
        info_chips = info_chips.push(chip(&format!("⎇ {}", project.branch), tc.purple));
        // PR dot in info bar
        if let Some(sr) = project.sub_repos.first() {
            let pr_color = if sr.has_unmerged_pr { tc.blue } else { tc.text_muted };
            info_chips = info_chips.push(text("⬤").size(8).color(pr_color));
        }
        if project.dirty_files > 0 {
            info_chips = info_chips.push(chip(&format!("✎ {} uncommitted", project.dirty_files), tc.orange));
        } else {
            info_chips = info_chips.push(chip("✓ clean", tc.green));
        }
        if session.background_agents > 0 {
            info_chips = info_chips.push(chip(&format!("🤖 {} agents", session.background_agents), tc.green));
        }
        info_chips = info_chips.push(Space::new().width(Fill));
        info_chips = info_chips.push(text(project.path.display().to_string()).size(11).font(MONO_FONT).color(tc.text_muted));

        let info_bar = container(info_chips.padding([8, 16]))
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(t.bg_panel)),
                border: Border { color: t.border, width: 0.0, ..Default::default() },
                ..Default::default()
            });

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

        column![info_bar, rule::horizontal(1), terminal_view].into()
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
            tree = tree.push(
                button(row![
                    text(icon).size(12).font(MONO_FONT).color(color),
                    text(&entry.name).size(12).font(MONO_FONT).color(color),
                ].spacing(6))
                .on_press(Message::ToggleFileExpand(idx)).style(button::text)
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

        container(column![
            header, rule::horizontal(1),
            scrollable(column![
                text("Color Theme").size(12).color(tc.text_muted),
                themes,
            ].spacing(8).padding([16, 20])).height(Fill),
        ]).center_x(Fill).height(Fill).into()
    }

    fn theme(&self) -> Theme { Theme::Dark }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![iced::time::every(Duration::from_secs(10)).map(|_| Message::Tick)];
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
    info.sub_repos.iter().map(|r| session::SubRepoView {
        name: r.name.clone(),
        branch: r.branch.clone(),
        dirty_files: r.dirty_files,
        has_unmerged_pr: matches!(r.pr, git_info::PrStatus::Open(_)),
    }).collect()
}

/// Find claude binary path
fn which_claude() -> String {
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "claude".to_string())
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
