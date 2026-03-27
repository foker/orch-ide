# Claude Sessions

Native Rust desktop app for managing multiple Claude Code CLI sessions across projects. Three-panel layout: session cards sidebar, embedded terminal (alacritty backend), file explorer.

![mockup](mockup/index.html)

## Features

- Multiple project groups with multiple Claude sessions per project
- Embedded terminal with full color/unicode/TUI support
- Git info: branch, dirty files, nested sub-repos, PR status via `gh`
- Session status tracking via Claude Code hooks (idle → running → awaiting input)
- Persistence between restarts
- 6 color themes
- macOS .app bundle with custom icon

## Requirements

- Rust 1.91+
- Claude Code CLI (`claude`)
- `gh` CLI (for PR status)

## Development

```bash
cargo run
```

Logs: `tail -f /tmp/claude-sessions-debug.log`

## Release Build

```bash
cargo build --release
cp target/release/claude-sessions bundle/ClaudeSessions.app/Contents/MacOS/
open bundle/ClaudeSessions.app
```

Binary: `target/release/claude-sessions` (~13MB)

## macOS App Bundle

The `.app` bundle is in `bundle/ClaudeSessions.app`. After a release build, copy the binary there and launch with `open`.
