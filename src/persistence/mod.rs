use crate::session::{PipelineDef, ProjectGroup};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A folder that was affiliated with a Notion task, so the user can reopen the
/// same directory with the same session name later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLink {
    pub path: PathBuf,
    pub session_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    pub projects: Vec<ProjectGroup>,
    pub theme: String,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default = "default_true")]
    pub dangerously_skip_permissions: bool,
    #[serde(default)]
    pub quick_prompts: Vec<String>,
    #[serde(default = "default_true")]
    pub date_prefix_enabled: bool,
    #[serde(default)]
    pub notion_token: String,
    #[serde(default)]
    pub notion_database_id: Option<String>,
    #[serde(default)]
    pub notion_group_by_prop: Option<String>,
    #[serde(default = "default_agent_backend")]
    pub agent_backend: String,
    /// task id -> folders affiliated with that Notion task
    #[serde(default)]
    pub task_links: HashMap<String, Vec<TaskLink>>,
    #[serde(default)]
    pub pipelines: Vec<PipelineDef>,
}

fn default_sidebar_width() -> f32 { 280.0 }
fn default_true() -> bool { true }
fn default_agent_backend() -> String { "ClaudeCode".to_string() }

fn config_path() -> PathBuf {
    let dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude-sessions");
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

pub fn save(state: &AppState) {
    let path = config_path();
    if let Ok(json) = serde_json::to_string_pretty(state) {
        std::fs::write(&path, json).ok();
    }
}

pub fn load() -> Option<AppState> {
    let path = config_path();
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_missing_new_fields_gets_defaults() {
        let old = r#"{"projects":[],"theme":"Midnight","sidebar_width":280.0}"#;
        let state: AppState = serde_json::from_str(old).unwrap();
        assert_eq!(state.agent_backend, "ClaudeCode");
        assert_eq!(state.notion_token, "");
        assert!(state.notion_database_id.is_none());
        assert!(state.notion_group_by_prop.is_none());
    }

    #[test]
    fn roundtrip_preserves_new_fields() {
        let state = AppState {
            projects: vec![], theme: "Midnight".into(), sidebar_width: 280.0,
            groq_api_key: "".into(), dangerously_skip_permissions: true,
            quick_prompts: vec![], date_prefix_enabled: true,
            notion_token: "secret_tok".into(),
            notion_database_id: Some("db123".into()),
            notion_group_by_prop: Some("Status".into()),
            agent_backend: "OpenCode".into(),
            task_links: HashMap::new(),
            pipelines: vec![],
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.agent_backend, "OpenCode");
        assert_eq!(back.notion_database_id.as_deref(), Some("db123"));
    }
}
