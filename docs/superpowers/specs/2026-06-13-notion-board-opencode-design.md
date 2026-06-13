# Design: Notion Board integration + OpenCode backend

Date: 2026-06-13
Status: Approved (brainstorm)
Target app: OrchIDE (`claude-sessions`), Rust + iced 0.14, `src/main.rs`

## Summary

Two features for OrchIDE:

1. **Notion board** — connect to Notion, pick a database (board), and render an
   in-app task board that mirrors Notion. A new top-level **Board** screen sits
   beside the existing **IDE** screen. The session sidebar (left) persists across
   both screens. Opening a task spawns a "ghost" session card in the sidebar that
   can be turned into a real Claude/OpenCode session seeded from the task.
2. **OpenCode backend** — a global settings select `Claude Code | OpenCode`
   (default Claude Code) choosing which agent CLI new sessions launch.

Full Notion parity is the end goal, delivered in three phases. This document
specs all three; Phase 0 is detailed, Phases 1–2 are scoped at milestone level.

## Decisions (locked)

- **Auth**: paste an internal integration token (like the existing Groq key).
  No OAuth, no local redirect server in v1.
- **Scope**: full parity is the goal, reached incrementally (Ph0 read-only →
  Ph1 editable → Ph2 create/delete + rich text + all property types).
- **Ghost → session**: session name = task title; task body written to a
  `TASK-<id>.md` file in the chosen project so the agent can read it. No prompt
  injection.
- **Agent backend**: one global select, default Claude Code.

## Current architecture (relevant facts)

- Single `App` struct, `Message` enum, `fn view()` dispatcher in `src/main.rs`.
- `view()` renders a 3-column `row!`: `view_sessions()` (left sidebar) | center
  (`view_terminal` / `view_settings` / `view_confirm_delete`, chosen by flags) |
  `view_explorer()` (right). A status bar sits below.
- Terminals: alacritty backend via `iced_term`, one per session, spawned in
  `spawn_session_terminal(pi, si, resume)`.
- `launch_claude: bool` decides agent-vs-plain-shell at spawn time (NOT persisted;
  session-time only). `which_claude()` resolves the binary via the login shell.
- Persistence: JSON at `~/.claude-sessions/config.json` via `persistence::AppState`
  (serde, `#[serde(default ...)]` for forward-compat). Secrets (Groq key) already
  stored plaintext there.
- Async work uses `Task::perform(async {...}, Message::Variant)` — see `git_info`
  fetches in `App::boot`.

## Components

### 1. Screen model (chrome)

- Add `enum Screen { Ide, Board }`; field `app.screen` (default `Ide`).
- `view()` ALWAYS renders `view_sessions()` as the left column (already true →
  sidebar persists for free). The right region switches on `app.screen`:
  - `Screen::Ide` → existing center + `view_explorer()` (unchanged layout).
  - `Screen::Board` → `view_board()` spanning the full center+explorer width;
    explorer hidden.
- Screen switch UI: a segmented control `[ IDE | Board ]` at the very top of the
  sidebar, above the `SESSIONS` header. `Message::SetScreen(Screen)`.
- `view_settings` / `view_confirm_delete` remain overlays reachable from either
  screen (they short-circuit the center as today).
- `screen` is NOT persisted in v1 (boot always lands on `Ide`). Cheap to add later.

### 2. Notion module — `src/notion/mod.rs` (new)

- `struct NotionClient { token: String }`. Async `reqwest` (already a dependency).
- Headers on every request: `Authorization: Bearer <token>`,
  `Notion-Version: 2022-06-28`, `Content-Type: application/json`.
- Calls (each wrapped in `Task::perform` → a `Message` variant, never blocking UI):
  - `POST /v1/search` filtered to `object=database` → list of databases for the picker.
  - `GET /v1/databases/{id}` → schema (`properties`), used for group-by selection
    and property rendering.
  - `POST /v1/databases/{id}/query` → pages (tasks), paginated via `next_cursor`.
  - Ph1: `PATCH /v1/pages/{id}` → update properties.
  - Ph2: `POST /v1/pages` (create), `PATCH /v1/pages/{id}` `archived=true` (delete),
    `GET/PATCH /v1/blocks/{id}/children` (page body / rich text).
- Errors: map non-2xx + transport errors into a `Result<_, NotionError>` carried
  by the result `Message`. Surface failures as a non-blocking banner in the board
  top bar (and keep last good data on screen).

Data model:

```rust
struct NotionDatabase { id: String, title: String, props: Vec<NotionProp> }
struct NotionProp { id: String, name: String, kind: PropKind }
enum PropKind {
    Title, RichText, Number, Checkbox, Url, Email, Phone, Date,
    Select { options: Vec<SelectOption> },
    MultiSelect { options: Vec<SelectOption> },
    Status { options: Vec<SelectOption> },
    People, Relation, Files, Formula, Rollup,
    CreatedTime, LastEditedTime, Unknown(String),
}
struct SelectOption { id: String, name: String, color: String }
struct NotionTask { id: String, title: String, props: HashMap<String, PropValue>, body: Option<Vec<Block>> }
enum PropValue { Text(String), Number(f64), Checkbox(bool), Date(..), Select(Option<SelectOption>),
                 MultiSelect(Vec<SelectOption>), People(Vec<String>), Url(String), Raw(serde_json::Value), Empty }
```

`PropKind::Unknown` / `PropValue::Raw` are the graceful-degradation escape hatch:
any property type not yet modelled renders read-only from raw JSON instead of
crashing. Phase 2 promotes the high-value ones to typed variants.

### 3. Board view — `view_board()` (Ph0 read-only)

- Top bar: database title, a **group-by** property selector (lists `Status` +
  `Select` properties; defaults to the first `Status`, else first `Select`),
  refresh button, error banner slot.
- Columns: one per option of the group-by property, plus a trailing "No <prop>"
  column for tasks with no value. Horizontal scroll. Each column is a vertical
  scroll of task cards.
- Task card: title + up to ~2 property chips (e.g. assignee, a select tag).
- Click a card → set `app.open_task = Some(task_id)` and `app.ghost_task = Some(task)`.
  A task-detail panel renders as an overlay over the board (same overlay mechanism
  as settings) showing all properties (typed where modelled, raw otherwise) and,
  once fetched, the page body. Close button → back to board.

### 4. Ghost card → session  (decision 3C)

- State: `app.ghost_task: Option<NotionTask>` and `app.ghost_target_project: Option<usize>`.
- When a task is open, the sidebar renders a **dashed "ghost" card** at the top of
  `view_sessions()` (visible on BOTH screens, persists across screen switches).
- Ghost card contents: task title, a target-project dropdown (default
  `app.active_project`), and a **Create session** button.
- On Create session (`Message::CreateSessionFromTask`):
  1. Resolve target project (selected, else `active_project`; if none, no-op + hint).
  2. Write `TASK-<short_id>.md` into the project dir: H1 title, properties list,
     body (markdown-rendered blocks if fetched), and the Notion page URL.
  3. `Session::new(task.title)`, push to project, `spawn_session_terminal(...)`.
  4. Clear `ghost_task` / `open_task`.
- Opening a different task replaces the ghost. Closing the task detail clears it.

### 5. OpenCode backend (decision 4A)

- `enum AgentBackend { ClaudeCode, OpenCode }`; field `app.agent_backend`
  (default `ClaudeCode`), persisted.
- Settings: a select (two buttons / radio row) `Claude Code | OpenCode`,
  `Message::SetAgentBackend(AgentBackend)`.
- `which_opencode()` mirrors `which_claude()`: login-shell `which opencode`,
  fallback `~/.opencode/bin/opencode`, then `opencode` on PATH.
- `spawn_session_terminal` branches on `agent_backend` (inside the existing
  `launch_agent` true branch):
  - `ClaudeCode` → current logic (`--name`, `--continue` on resume,
    `--dangerously-skip-permissions`, claude hooks configured).
  - `OpenCode` → program = `which_opencode()`, args = `[]` (cwd already set as
    `working_directory`, so the TUI starts in the project). No `--name`, no
    `--continue` (opencode manages its own sessions; resume = relaunch), no
    skip-permissions flag, and **claude hooks are NOT configured** — opencode does
    not emit them, so opencode session status stays at manual `Idle`. Document
    this limitation in-app (small note under the select).
- Rename internal `launch_claude` → `launch_agent` (means "launch agent vs plain
  shell"). Safe: not persisted. Update the new-session/new-project dialog labels
  to say "agent" instead of "claude".

### 6. Persistence — `AppState` additions

Add (all with serde defaults for forward/backward compat):

```rust
#[serde(default)] pub notion_token: String,
#[serde(default)] pub notion_database_id: Option<String>,
#[serde(default)] pub notion_group_by_prop: Option<String>,
#[serde(default = "default_agent_backend")] pub agent_backend: String, // "ClaudeCode" | "OpenCode"
```

`notion_token` is a secret stored plaintext, consistent with the existing
`groq_api_key`. Not ideal; acceptable for v1, flagged for a future keychain pass.
`save_state()` / `App::boot` extended to round-trip the new fields.

## Data flow

1. Boot: load `AppState`; if `notion_token` + `notion_database_id` present, fire a
   `Task::perform` board query → `Message::NotionTasksFetched`.
2. Settings: paste token → `Message::NotionTokenChanged` → on blur/submit fire
   `POST /v1/search` → `Message::NotionDatabasesFetched` populates the picker.
3. Pick database → store id, fetch schema (`GET /v1/databases/{id}`) →
   `Message::NotionSchemaFetched` → fetch tasks.
4. Board screen renders columns from schema + tasks. Click card → detail overlay +
   ghost card. Create session → file write + spawn + clear ghost.

## Phasing

- **Phase 0** (this milestone, end-to-end usable):
  Screen model, Notion module (search/schema/query only), read-only board,
  task-detail overlay, ghost card → session (md file), OpenCode backend, settings
  UI, persistence. Status tracking for opencode is intentionally absent.
- **Phase 1** (editable): change a card's group-by value (dropdown on the card or
  in detail) → `PATCH /v1/pages/{id}` with optimistic update + revert-on-error;
  inline edit of base property types in the detail panel (title, rich text, select,
  multi-select, status, date, checkbox, number). Drag-between-columns is the target
  UX; a status dropdown is the acceptable interim if iced DnD proves heavy.
- **Phase 2** (parity): create task (`POST /v1/pages`) and archive/delete; rich-text
  body editor over `/v1/blocks/{id}/children` (paragraph, headings, bullet, todo,
  code); remaining property types (relation, people, files, formula, rollup);
  board filters / sorts / grouping passed to the query endpoint.

Each phase ships independently and gets its own implementation plan.

## Testing

- Notion module: unit tests for JSON → model mapping (fixture responses for
  search / database schema / query / a task with mixed property types, including
  an unknown type that must fall back to `Raw`). Pure functions, no network.
- `which_opencode()` / agent-backend branch: unit test that `spawn` builds the
  expected `(program, args)` tuple for each backend (factor the program/args
  decision into a pure helper to make it testable without launching a terminal).
- Persistence: round-trip `AppState` with the new fields; load an old config
  (missing new fields) and assert defaults apply.
- Manual smoke: connect a real test database, render board, open a task, create a
  session, confirm `TASK-*.md` written and the chosen backend launches.

## Out of scope (v1 / Ph0)

- OAuth login. Multiple connected boards at once. Real-time webhook sync. Writing
  board changes back to Notion (that is Ph1+). opencode session status tracking.
  Keychain secret storage.
