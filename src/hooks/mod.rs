use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::session::SessionStatus;

#[derive(Debug, Deserialize)]
pub struct StatusPayload {
    pub status: Option<String>,
    #[serde(rename = "currentTask")]
    pub current_task: Option<String>,
    pub step: Option<u32>,
    #[serde(rename = "totalSteps")]
    pub total_steps: Option<u32>,
    pub etc: Option<String>,
}

impl StatusPayload {
    pub fn to_session_status(&self) -> Option<SessionStatus> {
        self.status.as_ref().map(|s| match s.as_str() {
            "idle" => SessionStatus::Idle,
            "running" => SessionStatus::Running,
            "awaiting-input" => SessionStatus::AwaitingInput,
            "done" => SessionStatus::Done,
            "error" => SessionStatus::Error,
            _ => SessionStatus::Running,
        })
    }
}

/// Get the base directory for claude-agents status files
pub fn status_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".claude-agents")
}

/// Read status file for a session
pub fn read_status(session_id: &str) -> Option<StatusPayload> {
    let path = status_dir().join(session_id).join("status.json");
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Create hook script for a session
pub fn create_hook_script(session_id: &str) -> Result<PathBuf, std::io::Error> {
    let dir = status_dir().join(session_id);
    std::fs::create_dir_all(&dir)?;

    let hook_path = dir.join("hook.sh");
    let status_file = dir.join("status.json");

    // Use printf with escaped quotes to generate valid JSON from bash
    let script = format!(
        r#"#!/bin/bash
SF="{status_file}"
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
E="$1"

INPUT=""
[ ! -t 0 ] && INPUT=$(cat 2>/dev/null || true)

TN=""
[ -n "$INPUT" ] && TN=$(echo "$INPUT" | grep -o '"tool_name":"[^"]*"' | head -1 | cut -d'"' -f4 2>/dev/null || true)

w() {{ printf '{{"status":"%s","currentTask":"%s","lastUpdate":"%s"}}' "$1" "$2" "$NOW" > "$SF"; }}
ws() {{ printf '{{"status":"%s","lastUpdate":"%s"}}' "$1" "$NOW" > "$SF"; }}

case "$E" in
  PostToolUse) w "running" "Tool: ${{TN:-working}}" ;;
  Stop) ws "awaiting-input" ;;
  UserPromptSubmit) w "running" "Processing prompt..." ;;
  Notification)
    MSG=$(echo "$INPUT" | grep -o '"message":"[^"]*"' | head -1 | cut -d'"' -f4 2>/dev/null || true)
    if echo "$MSG" | grep -qi "waiting"; then
      ws "awaiting-input"
    elif [ -n "$MSG" ]; then
      w "running" "$MSG"
    else
      ws "running"
    fi
    ;;
  *) ;;
esac
"#,
        status_file = status_file.display()
    );

    std::fs::write(&hook_path, &script)?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // Write initial status
    std::fs::write(&status_file, r#"{"status":"idle"}"#)?;

    Ok(hook_path)
}

/// Configure Claude hooks in project's .claude/settings.local.json
pub fn configure_claude_hooks(project_path: &Path, hook_script: &Path) -> Result<(), std::io::Error> {
    let claude_dir = project_path.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let settings_file = claude_dir.join("settings.local.json");
    let mut settings: serde_json::Value = if settings_file.exists() {
        let content = std::fs::read_to_string(&settings_file)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let hook_command = format!("bash \"{}\"", hook_script.display());

    let hooks = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .unwrap();

    for event in &["PostToolUse", "Stop", "Notification", "UserPromptSubmit"] {
        let arr = hooks
            .entry(*event)
            .or_insert(serde_json::json!([]))
            .as_array_mut()
            .unwrap();

        // Remove any old claude-agents hooks, then add current one
        arr.retain(|entry| {
            !entry.get("hooks")
                .and_then(|h| h.as_array())
                .map(|hks| hks.iter().any(|hh| {
                    hh.get("command").and_then(|c| c.as_str())
                        .map(|c| c.contains(".claude-agents")).unwrap_or(false)
                }))
                .unwrap_or(false)
        });

        arr.push(serde_json::json!({
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": format!("{} {}", hook_command, event)
            }]
        }));
    }

    std::fs::write(&settings_file, serde_json::to_string_pretty(&settings)?)?;
    Ok(())
}
