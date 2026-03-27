# 001 — Claude Agents VS Code Extension MVP

## Description
Create a VS Code extension that provides a panel with session cards for managing multiple Claude Code CLI sessions. Each session runs in a real VS Code terminal. The card panel shows session name, git branch, status, progress bar, and current task. Progress is reported via Claude Code hooks writing to status files, which the extension watches.

## Architecture
```
┌─────────────────────────────────────────────────┐
│  VS Code                                        │
│  ┌──────────────┐  ┌─────────────────────────┐  │
│  │  WebviewView  │  │  Terminal Panel          │  │
│  │  (cards list) │  │  (real terminals)        │  │
│  │               │  │                          │  │
│  │  [session 1]──┼──┤► terminal 1 (claude)     │  │
│  │  [session 2]──┼──┤► terminal 2 (claude)     │  │
│  │  [session 3]──┼──┤► terminal 3 (claude)     │  │
│  └──────────────┘  └─────────────────────────┘  │
│         ▲                                        │
│         │  file watch                            │
│         ▼                                        │
│  ~/.claude-agents/{session-id}/status.json       │
│         ▲                                        │
│         │  claude hooks write here               │
│         │                                        │
│  Claude Code (in terminal)                       │
│    hooks: PostToolUse, Stop, Notification         │
└─────────────────────────────────────────────────┘
```

## Plan

### Phase 1: Project Setup
- [x] 1.1 Initialize npm project with package.json (extension manifest)
- [x] 1.2 Configure TypeScript (tsconfig.json)
- [x] 1.3 Configure esbuild for bundling
- [x] 1.4 Create extension entry point (src/extension.ts) with activate/deactivate
- [x] 1.5 Add .gitignore, .vscodeignore
- [ ] 1.6 Verify extension loads in Extension Development Host

### Phase 2: Session Manager (Core Logic)
- [x] 2.1 Define session data model (ISession interface)
- [x] 2.2 Create SessionManager class (create, delete, list sessions)
- [x] 2.3 Session state machine: idle → running → awaiting-input → done → error
- [x] 2.4 Store sessions in memory (Map<id, ISession>)
- [x] 2.5 Event emitter for session state changes

### Phase 3: Terminal Integration
- [x] 3.1 Create terminal per session via vscode.window.createTerminal()
- [x] 3.2 Link terminal lifecycle to session (onDidCloseTerminal → cleanup)
- [x] 3.3 Launch claude in terminal via terminal.sendText()
- [x] 3.4 Detect git branch from session cwd
- [x] 3.5 Focus terminal on card click

### Phase 4: Progress Tracking via Hooks
- [x] 4.1 Design status.json schema
- [x] 4.2 Create hook shell script that writes status.json on Claude events
- [x] 4.3 Auto-configure Claude hooks when creating a session (write to .claude/settings.local.json)
- [x] 4.4 FileSystemWatcher on ~/.claude-agents/{id}/status.json
- [x] 4.5 Parse status updates → update session state + emit events
- [ ] 4.6 Add CLAUDE.md rule for session: "update progress after each plan step" (deferred — manual for now)

### Phase 5: WebviewView Panel (Cards UI)
- [x] 5.1 Register WebviewViewProvider for the cards panel
- [x] 5.2 HTML/CSS for card list (dark theme, matches VS Code)
- [x] 5.3 Card component: name, branch, status icon, progress bar, current task text
- [x] 5.4 "+" button to create new session (triggers input dialog)
- [x] 5.5 "X" button to kill session (kill process + close terminal)
- [x] 5.6 Click card → focus terminal
- [x] 5.7 Overall progress bar at the top of panel
- [x] 5.8 Message passing: webview ↔ extension (postMessage)
- [x] 5.9 Configurable panel position (sidebar left/right, panel bottom)

### Phase 6: Commands & Configuration
- [x] 6.1 Command: "Claude Agents: New Session"
- [x] 6.2 Command: "Claude Agents: Kill Session"
- [x] 6.3 Command: "Claude Agents: Focus Panel"
- [x] 6.4 Extension settings: panel position, hooks auto-configure toggle
- [x] 6.5 Register commands in package.json contributes

### Phase 7: Polish & Testing
- [x] 7.1 Error handling (claude not installed, hooks fail, etc.)
- [x] 7.2 Status icons with correct colors (green/yellow/grey/red)
- [ ] 7.3 Manual testing with real Claude Code sessions
- [ ] 7.4 README.md with screenshots and usage instructions
- [ ] 7.5 Package with vsce for local install

## Tech Stack
- TypeScript
- VS Code Extension API (WebviewView, Terminal, FileSystemWatcher)
- esbuild (bundler)
- HTML/CSS/JS (webview cards)

## Status: IN PROGRESS — MVP code ready, needs manual testing in VS Code
