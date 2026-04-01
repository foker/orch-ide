use crate::session::ProjectGroup;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

fn default_sidebar_width() -> f32 { 280.0 }
fn default_true() -> bool { true }

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
