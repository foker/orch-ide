# Notion Board + OpenCode — Phase 0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the full UX skeleton — connect to Notion by token, render a read-only board on a new "Board" screen beside the IDE, turn an opened task into a session seeded by a `TASK-*.md` file, and add a global Claude/OpenCode agent-backend select.

**Architecture:** All in the existing single-binary iced app (`src/main.rs` + a new `src/notion/` module). Pure data-mapping and command-building functions are unit-tested with `cargo test`; UI wiring is verified by a manual smoke checklist. Async Notion calls follow the existing `Task::perform(async {...}, Message::Variant)` pattern used for git info.

**Tech Stack:** Rust 2024, iced 0.14 (functional `application(boot, update, view)` API), reqwest (async, json — already a dependency), serde / serde_json, tokio.

Spec: `docs/superpowers/specs/2026-06-13-notion-board-opencode-design.md`

---

## File structure

- Create `src/notion/mod.rs` — Notion data model, pure JSON→model parsers, async client fns, `NotionError`. (`mod notion;` added to `src/main.rs`.)
- Modify `src/main.rs` — `AgentBackend` enum, `Screen` enum, new `App` fields, new `Message` variants + handlers + debug-name arms, `which_opencode()`, pure `build_agent_command()`, `task_to_markdown()`, `spawn_session_terminal` branch, `view()` dispatch, `view_board()`, task-detail overlay, ghost card in `view_sessions()`, settings additions.
- Modify `src/persistence/mod.rs` — new `AppState` fields with serde defaults + round-trip test.

Pure functions live next to their callers with `#[cfg(test)] mod tests` blocks (Rust convention; the repo has no separate tests dir).

Run all tests with: `cargo test` (from `rugs/orch-ide`). Build the app with: `cargo build`.

---

## Task 1: AgentBackend enum + pure `build_agent_command()`

Mechanical, independent quick win. Extracts the spawn program/args decision into a pure, testable function and adds the backend enum.

**Files:**
- Modify: `src/main.rs` (add enum near `AppTheme`; add `build_agent_command` + `which_opencode` near `which_claude`)

- [ ] **Step 1: Write the failing test**

Add at the bottom of `src/main.rs`:

```rust
#[cfg(test)]
mod agent_cmd_tests {
    use super::*;

    #[test]
    fn claude_fresh_uses_name_flag() {
        let (prog, args) = build_agent_command(
            AgentBackend::ClaudeCode, true, false,
            "/bin/claude", "/bin/opencode", "my task", true,
        );
        assert_eq!(prog, "/bin/claude");
        assert_eq!(args, vec!["--name", "my task", "--dangerously-skip-permissions"]);
    }

    #[test]
    fn claude_resume_uses_continue_shell() {
        let (prog, args) = build_agent_command(
            AgentBackend::ClaudeCode, true, true,
            "/bin/claude", "/bin/opencode", "my task", false,
        );
        assert_eq!(prog, "/bin/sh");
        assert_eq!(args[0], "-c");
        assert!(args[1].contains("--continue"));
    }

    #[test]
    fn opencode_runs_bare_in_cwd() {
        let (prog, args) = build_agent_command(
            AgentBackend::OpenCode, true, false,
            "/bin/claude", "/bin/opencode", "my task", true,
        );
        assert_eq!(prog, "/bin/opencode");
        assert!(args.is_empty());
    }

    #[test]
    fn plain_shell_when_agent_off() {
        let (prog, _args) = build_agent_command(
            AgentBackend::ClaudeCode, false, false,
            "/bin/claude", "/bin/opencode", "x", true,
        );
        assert!(prog.ends_with("sh") || prog.ends_with("zsh") || prog.contains("/"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test agent_cmd_tests 2>&1 | tail -20`
Expected: FAIL — `cannot find type AgentBackend` / `cannot find function build_agent_command`.

- [ ] **Step 3: Add the enum and the pure function**

Add near the `AppTheme` enum (after line ~35 in `src/main.rs`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentBackend { ClaudeCode, OpenCode }

impl AgentBackend {
    fn label(&self) -> &'static str {
        match self { AgentBackend::ClaudeCode => "Claude Code", AgentBackend::OpenCode => "OpenCode" }
    }
    fn from_str(s: &str) -> Self {
        match s { "OpenCode" => AgentBackend::OpenCode, _ => AgentBackend::ClaudeCode }
    }
    fn as_str(&self) -> &'static str {
        match self { AgentBackend::ClaudeCode => "ClaudeCode", AgentBackend::OpenCode => "OpenCode" }
    }
}

/// Pure decision: given backend + flags, return (program, args) for the terminal.
/// `launch_agent` false => plain login shell. Resume only meaningful for Claude.
fn build_agent_command(
    backend: AgentBackend,
    launch_agent: bool,
    resume: bool,
    claude_path: &str,
    opencode_path: &str,
    session_name: &str,
    skip_perms: bool,
) -> (String, Vec<String>) {
    if !launch_agent {
        let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        return (sh, vec![]);
    }
    match backend {
        AgentBackend::OpenCode => (opencode_path.to_string(), vec![]),
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test agent_cmd_tests 2>&1 | tail -20`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: AgentBackend enum + pure build_agent_command + which_opencode"
```

---

## Task 2: Wire `agent_backend` into App state + spawn

Replace the inline spawn logic with `build_agent_command`, rename `launch_claude` → `launch_agent`, add the `agent_backend` field.

**Files:**
- Modify: `src/main.rs` (`App` struct ~195-238, `Default` impl, `spawn_session_terminal` ~302-365, all `launch_claude` references)

- [ ] **Step 1: Rename field + add backend field**

In `struct App`: rename `launch_claude: bool` → `launch_agent: bool`, and add below it:

```rust
    agent_backend: AgentBackend,
```

In `impl Default for App`: rename `launch_claude: true,` → `launch_agent: true,` and add `agent_backend: AgentBackend::ClaudeCode,`.

Update every other `self.launch_claude` reference (in `update` handlers and `view_sessions`/`view_terminal` dialogs) to `self.launch_agent`. Find them with:
`grep -n launch_claude src/main.rs`
Also rename `Message::ToggleLaunchClaude` handler usage stays the same name for now (handler at ~430 sets `self.launch_agent = !self.launch_agent;`).

- [ ] **Step 2: Rewrite the program/args block in `spawn_session_terminal`**

Replace the `let (program, args) = if self.launch_claude { ... } else { ... };` block (≈ lines 310-326) with:

```rust
        let claude_path = which_claude();
        let opencode_path = which_opencode();
        let (program, args) = build_agent_command(
            self.agent_backend,
            self.launch_agent,
            resume,
            &claude_path,
            &opencode_path,
            &session_name,
            self.dangerously_skip_permissions,
        );
```

(`skip_flag` local is now unused — delete its declaration if the compiler warns.)

- [ ] **Step 3: Skip claude hooks for OpenCode**

In `spawn_session_terminal`, the hooks block (≈ lines 360-364) is guarded so opencode does not get claude hooks:

```rust
            if self.agent_backend == AgentBackend::ClaudeCode && self.launch_agent {
                let sid = &self.projects[pi].sessions[si].id;
                if let Ok(hp) = hooks::create_hook_script(sid) {
                    let _ = hooks::configure_claude_hooks(&self.projects[pi].path, &hp);
                }
            }
```

- [ ] **Step 4: Build to verify it compiles**

Run: `cargo build 2>&1 | tail -20`
Expected: compiles (warnings ok). No `launch_claude` left: `grep -c launch_claude src/main.rs` → `0`.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: spawn uses build_agent_command + agent_backend field"
```

---

## Task 3: Persist `agent_backend` + Notion fields

**Files:**
- Modify: `src/persistence/mod.rs` (AppState + defaults + test)
- Modify: `src/main.rs` (`save_state`, `App::boot` load)

- [ ] **Step 1: Write the failing round-trip test**

Add to the bottom of `src/persistence/mod.rs`:

```rust
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
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.agent_backend, "OpenCode");
        assert_eq!(back.notion_database_id.as_deref(), Some("db123"));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p claude-sessions persistence 2>&1 | tail -20` (or `cargo test persistence`)
Expected: FAIL — missing fields on `AppState`.

- [ ] **Step 3: Add fields + defaults**

In `src/persistence/mod.rs`, add to `struct AppState`:

```rust
    #[serde(default)]
    pub notion_token: String,
    #[serde(default)]
    pub notion_database_id: Option<String>,
    #[serde(default)]
    pub notion_group_by_prop: Option<String>,
    #[serde(default = "default_agent_backend")]
    pub agent_backend: String,
```

Add the default fn near the others:

```rust
fn default_agent_backend() -> String { "ClaudeCode".to_string() }
```

- [ ] **Step 4: Wire save + load in `src/main.rs`**

In `save_state()` add to the `AppState { ... }` literal:

```rust
            notion_token: self.notion_token.clone(),
            notion_database_id: self.notion_database_id.clone(),
            notion_group_by_prop: self.notion_group_by_prop.clone(),
            agent_backend: self.agent_backend.as_str().to_string(),
```

In `App::boot`, after the existing `app.date_prefix_enabled = state.date_prefix_enabled;` line, add:

```rust
            app.notion_token = state.notion_token;
            app.notion_database_id = state.notion_database_id;
            app.notion_group_by_prop = state.notion_group_by_prop;
            app.agent_backend = AgentBackend::from_str(&state.agent_backend);
```

Add these fields to `struct App` and its `Default` (placed near `groq_api_key`):

```rust
    notion_token: String,
    notion_database_id: Option<String>,
    notion_group_by_prop: Option<String>,
```

Default values: `notion_token: String::new(), notion_database_id: None, notion_group_by_prop: None,`
(`agent_backend` already added in Task 2.)

- [ ] **Step 5: Run tests + build**

Run: `cargo test persistence 2>&1 | tail -20`  → PASS (2 tests)
Run: `cargo build 2>&1 | tail -5` → compiles.

- [ ] **Step 6: Commit**

```bash
git add src/persistence/mod.rs src/main.rs
git commit -m "feat: persist agent_backend + notion token/db/group-by"
```

---

## Task 4: Notion data model + pure parsers

**Files:**
- Create: `src/notion/mod.rs`
- Modify: `src/main.rs` (add `mod notion;` near the other `mod` declarations at top)

- [ ] **Step 1: Write failing parser tests**

Create `src/notion/mod.rs` with model + empty parser stubs + tests (stubs return `Default`/empty so the file compiles but assertions fail):

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectOption { pub id: String, pub name: String, pub color: String }

#[derive(Debug, Clone, PartialEq)]
pub enum PropKind {
    Title, RichText, Number, Checkbox, Url, Email, Phone, Date,
    Select(Vec<SelectOption>), MultiSelect(Vec<SelectOption>), Status(Vec<SelectOption>),
    People, Relation, Files, Formula, Rollup, CreatedTime, LastEditedTime, Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotionProp { pub id: String, pub name: String, pub kind: PropKind }

#[derive(Debug, Clone)]
pub struct NotionDatabase { pub id: String, pub title: String, pub props: Vec<NotionProp> }

#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Text(String), Number(f64), Checkbox(bool), Date(String),
    Select(Option<SelectOption>), MultiSelect(Vec<SelectOption>),
    People(Vec<String>), Url(String), Raw(String), Empty,
}

#[derive(Debug, Clone)]
pub struct NotionTask {
    pub id: String,
    pub title: String,
    pub url: String,
    pub props: HashMap<String, PropValue>,
}

/// Parse the `results` array of POST /v1/search (databases only).
pub fn parse_databases(v: &serde_json::Value) -> Vec<NotionDatabase> {
    let _ = v; Vec::new() // stub
}

/// Parse GET /v1/databases/{id} into a schema.
pub fn parse_database(v: &serde_json::Value) -> Option<NotionDatabase> {
    let _ = v; None // stub
}

/// Parse one page object (an element of query `results`) into a task.
/// `props` is the parent DB schema, used to read property kinds.
pub fn parse_task(v: &serde_json::Value, props: &[NotionProp]) -> Option<NotionTask> {
    let _ = (v, props); None // stub
}

fn plain_text(arr: &serde_json::Value) -> String {
    arr.as_array().map(|a| a.iter()
        .filter_map(|t| t.get("plain_text").and_then(|s| s.as_str()))
        .collect::<String>()).unwrap_or_default()
}

fn select_opt(v: &serde_json::Value) -> Option<SelectOption> {
    let o = v.as_object()?;
    Some(SelectOption {
        id: o.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        name: o.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        color: o.get("color").and_then(|x| x.as_str()).unwrap_or("default").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Vec<NotionProp> {
        vec![
            NotionProp { id: "title".into(), name: "Name".into(), kind: PropKind::Title },
            NotionProp { id: "stat".into(), name: "Status".into(),
                kind: PropKind::Status(vec![SelectOption{id:"s1".into(),name:"Todo".into(),color:"gray".into()}]) },
        ]
    }

    #[test]
    fn parses_databases_list() {
        let v = json!({"results":[{
            "object":"database","id":"db1",
            "title":[{"plain_text":"My Board"}],
            "properties":{"Name":{"id":"title","type":"title","title":{}}}
        }]});
        let dbs = parse_databases(&v);
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].id, "db1");
        assert_eq!(dbs[0].title, "My Board");
    }

    #[test]
    fn parses_database_schema_kinds() {
        let v = json!({
            "id":"db1","title":[{"plain_text":"B"}],
            "properties":{
                "Name":{"id":"title","type":"title","title":{}},
                "Status":{"id":"stat","type":"status","status":{"options":[
                    {"id":"s1","name":"Todo","color":"gray"}]}}
            }
        });
        let db = parse_database(&v).unwrap();
        let by_name: HashMap<_,_> = db.props.iter().map(|p|(p.name.clone(),p.kind.clone())).collect();
        assert_eq!(by_name["Name"], PropKind::Title);
        assert!(matches!(by_name["Status"], PropKind::Status(_)));
    }

    #[test]
    fn parses_task_title_and_status() {
        let v = json!({
            "id":"pg1","url":"https://notion.so/pg1",
            "properties":{
                "Name":{"id":"title","type":"title","title":[{"plain_text":"Fix bug"}]},
                "Status":{"id":"stat","type":"status","status":{"id":"s1","name":"Todo","color":"gray"}}
            }
        });
        let t = parse_task(&v, &schema()).unwrap();
        assert_eq!(t.title, "Fix bug");
        assert_eq!(t.id, "pg1");
        match &t.props["stat"] {
            PropValue::Select(Some(o)) => assert_eq!(o.name, "Todo"),
            other => panic!("expected Select(Some), got {:?}", other),
        }
    }

    #[test]
    fn unknown_prop_type_falls_back_to_raw() {
        let v = json!({
            "id":"pg1","url":"",
            "properties":{
                "Name":{"id":"title","type":"title","title":[{"plain_text":"X"}]},
                "Weird":{"id":"w","type":"rollup","rollup":{"number":5}}
            }
        });
        let mut sch = schema();
        sch.push(NotionProp{id:"w".into(),name:"Weird".into(),kind:PropKind::Rollup});
        let t = parse_task(&v, &sch).unwrap();
        assert!(matches!(t.props.get("w"), Some(PropValue::Raw(_)) | Some(PropValue::Empty)));
    }
}
```

Add to the top of `src/main.rs` with the other `mod` lines: `mod notion;`

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p claude-sessions notion::tests 2>&1 | tail -30`
Expected: FAIL — 4 assertions fail (stubs return empty/None).

- [ ] **Step 3: Implement the parsers**

Replace the three stub functions:

```rust
fn prop_kind(type_str: &str, def: &serde_json::Value) -> PropKind {
    let opts = |key: &str| def.get(key)
        .and_then(|o| o.get("options"))
        .and_then(|o| o.as_array())
        .map(|a| a.iter().filter_map(select_opt).collect())
        .unwrap_or_default();
    match type_str {
        "title" => PropKind::Title,
        "rich_text" => PropKind::RichText,
        "number" => PropKind::Number,
        "checkbox" => PropKind::Checkbox,
        "url" => PropKind::Url,
        "email" => PropKind::Email,
        "phone_number" => PropKind::Phone,
        "date" => PropKind::Date,
        "select" => PropKind::Select(opts("select")),
        "multi_select" => PropKind::MultiSelect(opts("multi_select")),
        "status" => PropKind::Status(opts("status")),
        "people" => PropKind::People,
        "relation" => PropKind::Relation,
        "files" => PropKind::Files,
        "formula" => PropKind::Formula,
        "rollup" => PropKind::Rollup,
        "created_time" => PropKind::CreatedTime,
        "last_edited_time" => PropKind::LastEditedTime,
        other => PropKind::Unknown(other.to_string()),
    }
}

fn parse_props(v: &serde_json::Value) -> Vec<NotionProp> {
    v.get("properties").and_then(|p| p.as_object()).map(|obj| {
        obj.iter().map(|(name, def)| {
            let type_str = def.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
            NotionProp {
                id: def.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                name: name.clone(),
                kind: prop_kind(type_str, def),
            }
        }).collect()
    }).unwrap_or_default()
}

fn db_title(v: &serde_json::Value) -> String {
    let t = plain_text(v.get("title").unwrap_or(&serde_json::Value::Null));
    if t.is_empty() { "Untitled".to_string() } else { t }
}

pub fn parse_databases(v: &serde_json::Value) -> Vec<NotionDatabase> {
    v.get("results").and_then(|r| r.as_array()).map(|arr| {
        arr.iter()
            .filter(|d| d.get("object").and_then(|o| o.as_str()) == Some("database"))
            .map(|d| NotionDatabase {
                id: d.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                title: db_title(d),
                props: parse_props(d),
            }).collect()
    }).unwrap_or_default()
}

pub fn parse_database(v: &serde_json::Value) -> Option<NotionDatabase> {
    let id = v.get("id")?.as_str()?.to_string();
    Some(NotionDatabase { id, title: db_title(v), props: parse_props(v) })
}

pub fn parse_task(v: &serde_json::Value, props: &[NotionProp]) -> Option<NotionTask> {
    let id = v.get("id")?.as_str()?.to_string();
    let url = v.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
    let pobj = v.get("properties")?.as_object()?;
    let mut out: HashMap<String, PropValue> = HashMap::new();
    let mut title = String::new();
    for prop in props {
        // find the property entry in the page by matching name
        let entry = pobj.iter().find(|(_, def)| {
            def.get("id").and_then(|i| i.as_str()) == Some(prop.id.as_str())
        }).map(|(_, def)| def);
        let val = match (&prop.kind, entry) {
            (PropKind::Title, Some(def)) => {
                let t = plain_text(def.get("title").unwrap_or(&serde_json::Value::Null));
                title = t.clone();
                PropValue::Text(t)
            }
            (PropKind::RichText, Some(def)) =>
                PropValue::Text(plain_text(def.get("rich_text").unwrap_or(&serde_json::Value::Null))),
            (PropKind::Number, Some(def)) =>
                def.get("number").and_then(|n| n.as_f64()).map(PropValue::Number).unwrap_or(PropValue::Empty),
            (PropKind::Checkbox, Some(def)) =>
                PropValue::Checkbox(def.get("checkbox").and_then(|b| b.as_bool()).unwrap_or(false)),
            (PropKind::Url, Some(def)) =>
                PropValue::Url(def.get("url").and_then(|s| s.as_str()).unwrap_or("").to_string()),
            (PropKind::Date, Some(def)) =>
                def.get("date").and_then(|d| d.get("start")).and_then(|s| s.as_str())
                    .map(|s| PropValue::Date(s.to_string())).unwrap_or(PropValue::Empty),
            (PropKind::Select(_), Some(def)) =>
                PropValue::Select(def.get("select").and_then(select_opt)),
            (PropKind::Status(_), Some(def)) =>
                PropValue::Select(def.get("status").and_then(select_opt)),
            (PropKind::MultiSelect(_), Some(def)) =>
                PropValue::MultiSelect(def.get("multi_select").and_then(|a| a.as_array())
                    .map(|a| a.iter().filter_map(select_opt).collect()).unwrap_or_default()),
            (PropKind::People, Some(def)) =>
                PropValue::People(def.get("people").and_then(|a| a.as_array())
                    .map(|a| a.iter().filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
                    .unwrap_or_default()),
            (_, Some(def)) => PropValue::Raw(def.to_string()),
            (_, None) => PropValue::Empty,
        };
        out.insert(prop.id.clone(), val);
    }
    Some(NotionTask { id, title, url, props: out })
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test notion::tests 2>&1 | tail -20`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/notion/mod.rs src/main.rs
git commit -m "feat: notion data model + pure JSON parsers with tests"
```

---

## Task 5: Notion async client + Messages + boot/refresh wiring

**Files:**
- Modify: `src/notion/mod.rs` (async fns + `NotionError`)
- Modify: `src/main.rs` (Message variants, handlers, debug names, boot fetch, App fields for fetched data)

- [ ] **Step 1: Add async client functions to `src/notion/mod.rs`**

```rust
const NOTION_VERSION: &str = "2022-06-28";
const API: &str = "https://api.notion.com/v1";

pub type NotionResult<T> = Result<T, String>;

fn client(token: &str) -> reqwest::RequestBuilder {
    // placeholder; real builders created per-call below
    reqwest::Client::new().get(API).bearer_auth(token)
}

async fn post(token: &str, path: &str, body: serde_json::Value) -> NotionResult<serde_json::Value> {
    let resp = reqwest::Client::new()
        .post(format!("{API}{path}"))
        .bearer_auth(token)
        .header("Notion-Version", NOTION_VERSION)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("request failed");
        return Err(format!("Notion {}: {}", status.as_u16(), msg));
    }
    Ok(v)
}

async fn get(token: &str, path: &str) -> NotionResult<serde_json::Value> {
    let resp = reqwest::Client::new()
        .get(format!("{API}{path}"))
        .bearer_auth(token)
        .header("Notion-Version", NOTION_VERSION)
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("request failed");
        return Err(format!("Notion {}: {}", status.as_u16(), msg));
    }
    Ok(v)
}

pub async fn list_databases(token: String) -> NotionResult<Vec<NotionDatabase>> {
    let body = serde_json::json!({ "filter": { "property": "object", "value": "database" } });
    let v = post(&token, "/search", body).await?;
    Ok(parse_databases(&v))
}

pub async fn fetch_schema(token: String, db_id: String) -> NotionResult<NotionDatabase> {
    let v = get(&token, &format!("/databases/{db_id}")).await?;
    parse_database(&v).ok_or_else(|| "could not parse database schema".to_string())
}

pub async fn query_tasks(token: String, db_id: String, props: Vec<NotionProp>) -> NotionResult<Vec<NotionTask>> {
    let mut tasks = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut body = serde_json::json!({ "page_size": 100 });
        if let Some(c) = &cursor { body["start_cursor"] = serde_json::Value::String(c.clone()); }
        let v = post(&token, &format!("/databases/{db_id}/query"), body).await?;
        if let Some(arr) = v.get("results").and_then(|r| r.as_array()) {
            for page in arr { if let Some(t) = parse_task(page, &props) { tasks.push(t); } }
        }
        if v.get("has_more").and_then(|h| h.as_bool()) == Some(true) {
            cursor = v.get("next_cursor").and_then(|c| c.as_str()).map(String::from);
            if cursor.is_none() { break; }
        } else { break; }
    }
    Ok(tasks)
}
```

(Delete the unused `client` helper if it warns — kept here only as a note; remove it.)

- [ ] **Step 2: Add App fields for fetched Notion data**

In `struct App` (+ `Default`):

```rust
    // Notion runtime (not persisted)
    notion_databases: Vec<notion::NotionDatabase>,
    notion_schema: Option<notion::NotionDatabase>,
    notion_tasks: Vec<notion::NotionTask>,
    notion_error: Option<String>,
    notion_loading: bool,
```

Defaults: `notion_databases: Vec::new(), notion_schema: None, notion_tasks: Vec::new(), notion_error: None, notion_loading: false,`

- [ ] **Step 3: Add Message variants**

In `enum Message` add:

```rust
    // Notion
    NotionTokenChanged(String),
    NotionConnect,                       // fire list_databases
    NotionDatabasesFetched(Result<Vec<notion::NotionDatabase>, String>),
    NotionSelectDatabase(String),        // db id
    NotionSchemaFetched(Result<notion::NotionDatabase, String>),
    NotionTasksFetched(Result<Vec<notion::NotionTask>, String>),
    NotionRefresh,
    NotionSetGroupBy(String),            // prop id
```

Add matching arms to `message_label` (the `&str` debug match near line 1879) — one per variant, e.g. `Message::NotionConnect => "NotionConnect",` etc. (all 8).

- [ ] **Step 4: Add handlers in `update`**

```rust
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
                        // default group-by = first Status, else first Select
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
                if let (Some(db), tok) = (self.notion_schema.clone(), self.notion_token.clone()) {
                    self.notion_loading = true;
                    return Task::perform(notion::query_tasks(tok, db.id.clone(), db.props.clone()), Message::NotionTasksFetched);
                }
                Task::none()
            }
            Message::NotionSetGroupBy(pid) => { self.notion_group_by_prop = Some(pid); self.save_state(); Task::none() }
```

`NotionProp` needs `Clone` (already derived). `NotionDatabase` needs `Clone` (already derived).

- [ ] **Step 5: Boot auto-fetch**

In `App::boot`, after loading persisted notion fields, push a board fetch when token + db present:

```rust
        if !app.notion_token.is_empty() {
            if let Some(db_id) = app.notion_database_id.clone() {
                let tok = app.notion_token.clone();
                boot_tasks.push(Task::perform(notion::fetch_schema(tok, db_id), Message::NotionSchemaFetched));
            }
        }
```

(Place before `(app, Task::batch(boot_tasks))`.)

- [ ] **Step 6: Build**

Run: `cargo build 2>&1 | tail -25`
Expected: compiles (warnings ok). Fix any missing `message_label` arm the compiler flags (the match must be exhaustive).

- [ ] **Step 7: Commit**

```bash
git add src/notion/mod.rs src/main.rs
git commit -m "feat: notion async client + fetch wiring (search/schema/query)"
```

---

## Task 6: Screen model + sidebar segmented toggle

**Files:**
- Modify: `src/main.rs` (`Screen` enum, App field, `SetScreen` message, `view()` dispatch, segmented control in `view_sessions`)

- [ ] **Step 1: Add the enum + field + message**

Near `AgentBackend`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen { Ide, Board }
```

`struct App`: add `screen: Screen,`  ·  `Default`: `screen: Screen::Ide,`
`enum Message`: add `SetScreen(Screen),`  ·  `message_label`: `Message::SetScreen(_) => "SetScreen",`
`update` handler:

```rust
            Message::SetScreen(s) => { self.screen = s; Task::none() }
```

- [ ] **Step 2: Dispatch the right region in `view()`**

In `fn view()`, replace the `main` row construction so the center+right depend on `self.screen`. Keep the sidebar column unchanged. Replace the existing `row![ sidebar, center, explorer ]` with:

```rust
        let right: Element<'_, Message> = match self.screen {
            Screen::Ide => row![
                container(center).width(Fill).height(Fill)
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(c(0x17, 0x1b, 0x21))), ..Default::default()
                    }),
                container(self.view_explorer()).width(260).height(Fill)
                    .style(move |_: &Theme| styled_panel(&tc3)),
            ].into(),
            Screen::Board => container(self.view_board()).width(Fill).height(Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(c(0x17, 0x1b, 0x21))), ..Default::default()
                }).into(),
        };

        let main = row![
            container(self.view_sessions()).width(self.sidebar_width).height(Fill)
                .style(move |_: &Theme| styled_panel(&tc1)),
            right,
        ];
```

Note: when `show_settings`/`confirm_delete` are active, `center` already overrides — those overlays should show on the IDE screen. In Board screen they are not reachable except settings; keep settings working by letting `Screen::Board` still honor `self.show_settings`: change the Board arm to:

```rust
            Screen::Board => {
                let inner: Element<'_, Message> = if self.show_settings { self.view_settings() } else { self.view_board() };
                container(inner).width(Fill).height(Fill)
                    .style(move |_: &Theme| container::Style {
                        background: Some(Background::Color(c(0x17,0x1b,0x21))), ..Default::default()
                    }).into()
            }
```

- [ ] **Step 3: Add the segmented toggle at the top of `view_sessions`**

In `view_sessions`, before the `header` container, build a screen switch and prepend it to the returned column:

```rust
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
```

Then include `switcher` as the first child of the sidebar's outer `column!` (above `header`). Find where `view_sessions` composes its final `column![header, ...]` / scrollable and put `switcher` first.

- [ ] **Step 4: Build + run**

Run: `cargo build 2>&1 | tail -10` → compiles.
Run app, click `Board` / `IDE` — region switches, sidebar stays. (`view_board` is a stub until Task 7 — add a temporary `fn view_board(&self) -> Element<'_, Message> { text("board").into() }` to compile; Task 7 replaces it.)

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: Screen enum + IDE/Board sidebar toggle + view dispatch"
```

---

## Task 7: Read-only board view

**Files:**
- Modify: `src/main.rs` (replace the `view_board` stub; add a pure `group_tasks` helper + test)

- [ ] **Step 1: Write a failing test for grouping**

Add to `src/main.rs` tests:

```rust
#[cfg(test)]
mod board_tests {
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
        // expect 3 columns: Todo, Done, "(none)"
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].0, "Todo"); assert_eq!(cols[0].1.len(), 1);
        assert_eq!(cols[2].0, "(none)"); assert_eq!(cols[2].1.len(), 1);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test board_tests 2>&1 | tail -15`
Expected: FAIL — `group_tasks` not found.

- [ ] **Step 3: Implement `group_tasks` + `view_board`**

```rust
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
```

`view_board`:

```rust
    fn view_board(&self) -> Element<'_, Message> {
        let tc = self.tc();

        // No DB connected yet → hint to open settings
        let Some(schema) = self.notion_schema.as_ref() else {
            return container(column![
                text("No Notion board connected").size(14).color(tc.text_primary),
                text("Open Settings → Notion, paste a token and pick a database.").size(11).color(tc.text_muted),
                button(text("Open Settings").size(12)).on_press(Message::ToggleSettings).style(button::secondary).padding([6,12]),
            ].spacing(10).align_x(iced::Alignment::Center)).center_x(Fill).center_y(Fill).into();
        };

        // group-by options
        let gp_id = self.notion_group_by_prop.clone().unwrap_or_default();
        let options: Vec<notion::SelectOption> = schema.props.iter()
            .find(|p| p.id == gp_id)
            .map(|p| match &p.kind {
                notion::PropKind::Status(o) | notion::PropKind::Select(o) => o.clone(),
                _ => vec![],
            }).unwrap_or_default();

        // top bar: title + group-by selector + refresh + error
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
        let topbar = container(row![
            text(schema.title.clone()).size(14).color(tc.text_primary),
            Space::new().width(Fill),
            groupby_row,
            button(text("⟳").size(13).color(tc.text_muted)).on_press(Message::NotionRefresh).style(button::text).padding(2),
        ].spacing(12).align_y(iced::Alignment::Center)).padding([10, 16]);

        let err_banner: Element<'_, Message> = if let Some(e) = &self.notion_error {
            container(text(format!("⚠ {e}")).size(11).color(tc.red)).padding([4,16]).into()
        } else { Space::new().height(0).into() };

        // columns
        let cols = group_tasks(&self.notion_tasks, &gp_id, &options);
        let mut board_row = Row::new().spacing(12).padding(12);
        for (label, tasks) in cols {
            let mut col = Column::new().spacing(8).padding(8);
            col = col.push(text(format!("{}  ({})", label, tasks.len())).size(11).color(tc.text_muted));
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

        column![ topbar, err_banner, scrollable(board_row).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())).height(Fill) ].into()
    }
```

`Message::NotionOpenTask(String)` is added in Task 8 — to compile this task standalone, temporarily use `Message::NotionRefresh` for the card `on_press`, then switch to `NotionOpenTask` in Task 8. (Or implement Task 8's message first.)

- [ ] **Step 4: Run tests + build**

Run: `cargo test board_tests 2>&1 | tail -10` → PASS
Run: `cargo build 2>&1 | tail -10` → compiles.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: read-only Notion board view + group_tasks helper"
```

---

## Task 8: Task-detail overlay + open task

**Files:**
- Modify: `src/main.rs` (`open_task` field, `NotionOpenTask`/`NotionCloseTask` messages, `view_task_detail`, dispatch in Board arm)

- [ ] **Step 1: Add state + messages**

`struct App`: `open_task: Option<String>,`  ·  Default `None`.
`enum Message`: `NotionOpenTask(String), NotionCloseTask,`  · `message_label` arms for both.
`update`:

```rust
            Message::NotionOpenTask(id) => {
                self.open_task = Some(id.clone());
                if let Some(t) = self.notion_tasks.iter().find(|t| t.id == id).cloned() {
                    self.ghost_task = Some(t);                       // wired fully in Task 9
                    self.ghost_target_project = self.active_project; // Task 9 field
                }
                Task::none()
            }
            Message::NotionCloseTask => { self.open_task = None; self.ghost_task = None; Task::none() }
```

(If Task 9 not yet done, the `ghost_*` lines won't compile — implement Task 9's fields first, or temporarily comment the two ghost lines and re-add in Task 9. Recommended: do Task 9 fields before this step.)

Switch the board card `on_press` (Task 7) to `Message::NotionOpenTask(tid)`.

- [ ] **Step 2: Implement `view_task_detail`**

```rust
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
                let val = task.props.get(&p.id);
                let rendered = match val {
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

        container(column![header, rule::horizontal(1), scrollable(props_col).height(Fill)]).into()
    }
```

- [ ] **Step 3: Show detail over board**

In `view()`'s `Screen::Board` arm, prefer settings > task detail > board:

```rust
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
                        background: Some(Background::Color(c(0x17,0x1b,0x21))), ..Default::default()
                    }).into()
            }
```

- [ ] **Step 4: Build**

Run: `cargo build 2>&1 | tail -12` → compiles.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: Notion task-detail overlay + open/close task"
```

---

## Task 9: Ghost card → session (with TASK md file)

**Files:**
- Modify: `src/main.rs` (`ghost_task`/`ghost_target_project` fields, `task_to_markdown` + test, ghost card in `view_sessions`, `CreateSessionFromTask` + `GhostSetProject` messages/handlers)

- [ ] **Step 1: Write failing test for `task_to_markdown`**

```rust
#[cfg(test)]
mod md_tests {
    use super::*;
    use notion::*;
    use std::collections::HashMap;

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
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test md_tests 2>&1 | tail -10` → FAIL (`task_to_markdown` not found).

- [ ] **Step 3: Implement `task_to_markdown`**

```rust
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
```

- [ ] **Step 4: Add fields + messages + handlers**

`struct App`: `ghost_task: Option<notion::NotionTask>,`  ·  `ghost_target_project: Option<usize>,`  · Defaults `None, None`.
`enum Message`: `GhostSetProject(usize), CreateSessionFromTask,`  · `message_label` arms.
`update`:

```rust
            Message::GhostSetProject(pi) => { self.ghost_target_project = Some(pi); Task::none() }
            Message::CreateSessionFromTask => {
                let Some(task) = self.ghost_task.clone() else { return Task::none(); };
                let pi = match self.ghost_target_project.or(self.active_project) {
                    Some(pi) if pi < self.projects.len() => pi,
                    _ => { self.notion_error = Some("Pick a target project for the session".into()); return Task::none(); }
                };
                // write TASK-<id>.md
                let schema_props = self.notion_schema.as_ref().map(|s| s.props.clone()).unwrap_or_default();
                let md = task_to_markdown(&task, &schema_props);
                let fname = format!("TASK-{}.md", &task.id.replace('-', "")[..task.id.replace('-', "").len().min(8)]);
                let path = self.projects[pi].path.join(&fname);
                let _ = std::fs::write(&path, md);
                // create + spawn session
                let name = if task.title.is_empty() { fname.clone() } else { task.title.clone() };
                self.projects[pi].sessions.push(Session::new(name));
                let si = self.projects[pi].sessions.len() - 1;
                self.active_project = Some(pi);
                self.active_session = Some((pi, si));
                self.spawn_session_terminal(pi, si, false);
                self.ghost_task = None; self.open_task = None;
                self.screen = Screen::Ide;     // jump to the terminal
                self.save_state();
                Task::none()
            }
```

- [ ] **Step 5: Render ghost card in `view_sessions`**

Insert, right after `switcher` (Task 6) and before `header`, a ghost block when present:

```rust
        let ghost: Element<'_, Message> = if let Some(gt) = &self.ghost_task {
            let mut proj_picker = Row::new().spacing(4);
            for (pi, p) in self.projects.iter().enumerate() {
                let active = self.ghost_target_project == Some(pi) || (self.ghost_target_project.is_none() && self.active_project == Some(pi));
                proj_picker = proj_picker.push(
                    button(text(p.name.clone()).size(9).color(if active { tc.text_primary } else { tc.text_muted }))
                        .on_press(Message::GhostSetProject(pi))
                        .style(if active { button::secondary } else { button::text }).padding([2,6])
                );
            }
            let tcg = tc.clone();
            container(column![
                text("NEW SESSION FROM TASK").size(8).color(tc.text_muted),
                text(gt.title.clone()).size(12).color(tc.text_primary),
                scrollable(proj_picker).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())),
                button(text("＋ Create session").size(11).color(tc.green))
                    .on_press(Message::CreateSessionFromTask).style(button::text).padding([2,0]),
            ].spacing(6).padding(10))
            .style(move |_: &Theme| container::Style {
                border: Border { color: tcg.border_active, width: 1.0, radius: 6.0.into(), ..Default::default() },
                ..styled_panel(&tcg)
            }).padding(6).into()
        } else { Space::new().height(0).into() };
```

Add `ghost` as a child of the sidebar column right after `switcher`.

- [ ] **Step 6: Run tests + build + manual check**

Run: `cargo test md_tests 2>&1 | tail -10` → PASS
Run: `cargo build 2>&1 | tail -10` → compiles.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat: ghost card -> session from Notion task (writes TASK md)"
```

---

## Task 10: Settings UI — agent backend + Notion connect

**Files:**
- Modify: `src/main.rs` (`view_settings` additions)

- [ ] **Step 1: Add agent-backend select to settings**

In `view_settings`, in `options_section` (or a new section above themes), add:

```rust
            Space::new().height(12),
            text("Agent Backend").size(12).color(tc.text_muted),
            row![
                button(text("Claude Code").size(11)
                    .color(if self.agent_backend == AgentBackend::ClaudeCode { tc.text_primary } else { tc.text_muted }))
                    .on_press(Message::SetAgentBackend(AgentBackend::ClaudeCode))
                    .style(if self.agent_backend == AgentBackend::ClaudeCode { button::secondary } else { button::text }).padding([4,12]),
                button(text("OpenCode").size(11)
                    .color(if self.agent_backend == AgentBackend::OpenCode { tc.text_primary } else { tc.text_muted }))
                    .on_press(Message::SetAgentBackend(AgentBackend::OpenCode))
                    .style(if self.agent_backend == AgentBackend::OpenCode { button::secondary } else { button::text }).padding([4,12]),
            ].spacing(6),
            text("OpenCode sessions have no live status tracking (no hooks).").size(9).color(tc.text_muted),
```

`enum Message`: `SetAgentBackend(AgentBackend),`  · `message_label` arm.
`update`: `Message::SetAgentBackend(b) => { self.agent_backend = b; self.save_state(); Task::none() }`

- [ ] **Step 2: Add Notion section to settings**

```rust
            Space::new().height(16),
            text("Notion").size(12).color(tc.text_muted),
            text_input("Notion integration token (secret_...)", &self.notion_token)
                .on_input(Message::NotionTokenChanged)
                .on_submit(Message::NotionConnect)
                .size(12).padding(6),
            button(text(if self.notion_loading { "Connecting…" } else { "Connect / List databases" }).size(11).color(tc.blue))
                .on_press(Message::NotionConnect).style(button::text).padding([2,0]),
```

Then a database picker list when `self.notion_databases` is non-empty:

```rust
            {
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
                dbs
            },
```

(If the `options_section` is a fixed `column![...]` macro, convert the relevant part to a `Column::new()` builder so these conditional children fit, or place the Notion section as a separate `Column` pushed into the settings scrollable alongside `options_section`.)

- [ ] **Step 2b: Show notion error in settings**

Below the picker:

```rust
            if let Some(e) = &self.notion_error {
                text(format!("⚠ {e}")).size(10).color(tc.red)
            } else { text("").size(1) },
```

- [ ] **Step 3: Build**

Run: `cargo build 2>&1 | tail -12` → compiles.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: settings — agent backend select + Notion connect/db picker"
```

---

## Task 11: Manual smoke test + release notes

No code changes; verification only.

- [ ] **Step 1: Build & launch**

Run: `cargo run 2>&1 | tail -5` (logs: `tail -f /tmp/claude-sessions-debug.log`)

- [ ] **Step 2: OpenCode backend**

1. Settings → Agent Backend → OpenCode.
2. Open/create a project, add a session. Confirm the terminal launches `opencode` in the project dir.
3. Switch back to Claude Code, add a session, confirm `claude --name ...` launches.

- [ ] **Step 3: Notion connect**

1. In Notion: create an internal integration, copy the token, share a database with it.
2. Settings → Notion → paste token → Connect. The database appears in the picker.
3. Select it. Switch to the **Board** screen. Columns render grouped by the default Status/Select property; switch group-by; refresh.

- [ ] **Step 4: Ghost → session**

1. Click a task card → detail overlay shows properties; the sidebar shows a dashed ghost card (visible on both IDE and Board screens).
2. Pick a target project on the ghost card → Create session.
3. Confirm a `TASK-*.md` exists in the project dir with the title/props, a new session spawned in the chosen project, and the app jumped to the IDE screen with the terminal running the selected backend.

- [ ] **Step 5: Persistence**

1. Quit and relaunch. Token + selected DB persist; the board auto-loads. Agent backend persists.

- [ ] **Step 6: Commit any doc/version bumps**

```bash
git add -A
git commit -m "chore: Phase 0 smoke verified (Notion board + OpenCode)"
```

---

## Self-review notes (author)

- **Spec coverage:** Screen model (T6), Notion module + parsers (T4) + client (T5), read-only board (T7), task detail (T8), ghost→session md (T9), OpenCode backend (T1-2) + persistence (T3), settings UI (T10), manual smoke (T11). All Phase-0 spec sections mapped.
- **Type consistency:** `AgentBackend`, `Screen`, `notion::{NotionDatabase, NotionProp, PropKind, PropValue, NotionTask, SelectOption}`, `build_agent_command`, `group_tasks`, `task_to_markdown` referenced consistently across tasks. Every new `Message` variant gets a `message_label` arm (compiler-enforced exhaustiveness).
- **Cross-task ordering caveat:** T7 card `on_press` uses `NotionOpenTask` (defined T8) and T8 references `ghost_*` (defined T9). To keep each commit compiling, implement the small state/message additions of T8 and T9 fields before wiring T7/T8 press handlers, OR use the noted temporary placeholders. Recommended execution order if strict per-commit green is required: T1, T2, T3, T4, T5, T6, T9(fields+md), T8, T7, T10, T11.
- **iced 0.14 API:** mirrors existing usage in `main.rs` (`button`, `text`, `container`, `scrollable`, `Row::new`, `column!`, `styled_panel`, `tc()`). Verify `scrollable::Direction::Horizontal(scrollable::Scrollbar::new())` matches the version in `Cargo.lock` (iced 0.14); adjust the scrollbar constructor if the API differs.
