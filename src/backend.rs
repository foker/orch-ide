//! Agent backend selection and command building.
//!
//! Pure, UI-free logic for choosing which CLI to launch (Claude Code, OpenCode,
//! Codex, Xiaomi MiMo) and resolving their binary paths.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentBackend { ClaudeCode, OpenCode, Codex, XiaomiMimo }

/// Model id the Xiaomi MiMo backend pins opencode to (provider/model).
pub const MIMO_MODEL: &str = "mimo/mimo-v2.5-pro";

impl AgentBackend {
    pub fn all() -> &'static [AgentBackend] {
        &[AgentBackend::ClaudeCode, AgentBackend::OpenCode, AgentBackend::Codex, AgentBackend::XiaomiMimo]
    }
    pub fn label(&self) -> &'static str {
        match self {
            AgentBackend::ClaudeCode => "Claude Code",
            AgentBackend::OpenCode => "OpenCode",
            AgentBackend::Codex => "Codex",
            AgentBackend::XiaomiMimo => "Xiaomi MiMo",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "OpenCode" => AgentBackend::OpenCode,
            "Codex" => AgentBackend::Codex,
            "XiaomiMimo" => AgentBackend::XiaomiMimo,
            _ => AgentBackend::ClaudeCode,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentBackend::ClaudeCode => "ClaudeCode",
            AgentBackend::OpenCode => "OpenCode",
            AgentBackend::Codex => "Codex",
            AgentBackend::XiaomiMimo => "XiaomiMimo",
        }
    }
}

/// Pure decision: given backend + flags, return (program, args) for the terminal.
/// `launch_agent` false => plain login shell. Resume only meaningful for Claude.
pub fn build_agent_command(
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

/// Find claude binary path (login shell `which`, then common fallbacks).
pub fn which_claude() -> String {
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
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "claude".to_string())
}

/// Find opencode binary path (mirrors which_claude).
pub fn which_opencode() -> String {
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
pub fn which_codex() -> String {
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

/// Prepend $HOME/.local/bin to a PATH string if not already present.
pub fn prepend_local_bin(path: String) -> String {
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
