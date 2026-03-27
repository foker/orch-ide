import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';
import { ISession, SessionStatus, StatusFilePayload, SessionProgress } from './types';

export class SessionManager {
  private sessions = new Map<string, ISession>();
  private terminals = new Map<string, vscode.Terminal>();
  private watchers = new Map<string, fs.FSWatcher>();
  private branchIntervals = new Map<string, NodeJS.Timeout>();

  private readonly _onDidChangeSessions = new vscode.EventEmitter<ISession[]>();
  public readonly onDidChangeSessions = this._onDidChangeSessions.event;

  private readonly statusDir: string;

  constructor() {
    this.statusDir = path.join(
      process.env.HOME || process.env.USERPROFILE || '',
      '.claude-agents'
    );
    if (!fs.existsSync(this.statusDir)) {
      fs.mkdirSync(this.statusDir, { recursive: true });
    }
  }

  getSessions(): ISession[] {
    return Array.from(this.sessions.values());
  }

  getSession(id: string): ISession | undefined {
    return this.sessions.get(id);
  }

  async createSession(name: string, cwd: string, prompt?: string): Promise<ISession> {
    const id = crypto.randomUUID().slice(0, 8);
    const branch = await this.detectBranch(cwd);

    const session: ISession = {
      id,
      name,
      cwd,
      branch,
      status: 'idle',
      progress: null,
      createdAt: new Date().toISOString(),
    };

    this.sessions.set(id, session);

    // Create status directory for this session
    const sessionStatusDir = path.join(this.statusDir, id);
    if (!fs.existsSync(sessionStatusDir)) {
      fs.mkdirSync(sessionStatusDir, { recursive: true });
    }

    // Setup hooks for this session
    await this.setupHooks(id, sessionStatusDir);
    await this.configureClaudeHooks(id, cwd, path.join(sessionStatusDir, 'hook.sh'));

    // Create terminal
    const terminal = vscode.window.createTerminal({
      name: `Claude: ${name}`,
      cwd,
      iconPath: new vscode.ThemeIcon('hubot'),
    });
    this.terminals.set(id, terminal);

    // Watch for terminal close
    const disposable = vscode.window.onDidCloseTerminal((t) => {
      if (t === terminal) {
        this.handleTerminalClosed(id);
        disposable.dispose();
      }
    });

    // Start file watcher for status updates
    this.watchStatus(id, sessionStatusDir);

    // Start branch polling
    this.startBranchPolling(id, cwd);

    // Launch claude if prompt provided
    if (prompt) {
      session.status = 'running';
      terminal.sendText(`claude -p ${this.shellEscape(prompt)}`);
    }

    terminal.show(false);
    this.emitChange();
    return session;
  }

  focusSession(id: string): void {
    const terminal = this.terminals.get(id);
    if (terminal) {
      terminal.show(false);
    }
  }

  killSession(id: string): void {
    const terminal = this.terminals.get(id);
    if (terminal) {
      terminal.dispose();
    }
    this.cleanup(id);
  }

  private handleTerminalClosed(id: string): void {
    const session = this.sessions.get(id);
    if (session) {
      session.status = 'done';
    }
    this.stopBranchPolling(id);
    this.stopWatcher(id);
    this.terminals.delete(id);
    this.emitChange();
  }

  private watchStatus(id: string, statusDir: string): void {
    const statusFile = path.join(statusDir, 'status.json');

    // Create initial empty status file
    if (!fs.existsSync(statusFile)) {
      fs.writeFileSync(statusFile, JSON.stringify({ status: 'idle' }));
    }

    try {
      const watcher = fs.watch(statusFile, () => {
        this.handleStatusUpdate(id, statusFile);
      });
      this.watchers.set(id, watcher);
    } catch {
      // File watch may fail on some systems, that's ok
    }
  }

  private handleStatusUpdate(id: string, statusFile: string): void {
    const session = this.sessions.get(id);
    if (!session) return;

    try {
      const raw = fs.readFileSync(statusFile, 'utf-8');
      const data: StatusFilePayload = JSON.parse(raw);

      if (data.status) {
        session.status = data.status;
      }

      if (data.step !== undefined && data.totalSteps !== undefined) {
        session.progress = {
          step: data.step,
          totalSteps: data.totalSteps,
          currentTask: data.currentTask || '',
          etc: data.etc,
          lastUpdate: data.lastUpdate || new Date().toISOString(),
        };
      } else if (data.currentTask) {
        session.progress = {
          step: session.progress?.step || 0,
          totalSteps: session.progress?.totalSteps || 0,
          currentTask: data.currentTask,
          etc: data.etc,
          lastUpdate: data.lastUpdate || new Date().toISOString(),
        };
      }

      this.emitChange();
    } catch {
      // Ignore parse errors (file might be partially written)
    }
  }

  private async setupHooks(sessionId: string, statusDir: string): Promise<void> {
    const hookScript = path.join(statusDir, 'hook.sh');
    const statusFile = path.join(statusDir, 'status.json');

    // Hook script — event name passed as $1 argument
    const script = `#!/bin/bash
# Claude Agents hook script for session ${sessionId}
STATUS_FILE="${statusFile}"
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EVENT="\$1"

# Read stdin for context (Claude may pass JSON)
INPUT=""
if [ ! -t 0 ]; then
  INPUT=$(cat 2>/dev/null || true)
fi

# Try to extract tool name from stdin JSON
TOOL_NAME=""
if [ -n "$INPUT" ]; then
  TOOL_NAME=$(echo "$INPUT" | grep -o '"tool_name":"[^"]*"' | head -1 | cut -d'"' -f4 2>/dev/null || true)
fi

case "$EVENT" in
  PostToolUse)
    TASK_DESC="Tool: \${TOOL_NAME:-working}"
    echo "{\\"status\\":\\"running\\",\\"currentTask\\":\\"$TASK_DESC\\",\\"lastUpdate\\":\\"$NOW\\"}" > "$STATUS_FILE"
    ;;
  Notification)
    MSG=$(echo "$INPUT" | grep -o '"message":"[^"]*"' | head -1 | cut -d'"' -f4 2>/dev/null || true)
    if [ -n "$MSG" ]; then
      echo "{\\"status\\":\\"running\\",\\"currentTask\\":\\"$MSG\\",\\"lastUpdate\\":\\"$NOW\\"}" > "$STATUS_FILE"
    else
      echo "{\\"status\\":\\"running\\",\\"lastUpdate\\":\\"$NOW\\"}" > "$STATUS_FILE"
    fi
    ;;
  Stop)
    echo "{\\"status\\":\\"awaiting-input\\",\\"lastUpdate\\":\\"$NOW\\"}" > "$STATUS_FILE"
    ;;
  UserPromptSubmit)
    echo "{\\"status\\":\\"running\\",\\"currentTask\\":\\"Processing prompt...\\",\\"lastUpdate\\":\\"$NOW\\"}" > "$STATUS_FILE"
    ;;
  *)
    echo "{\\"status\\":\\"running\\",\\"lastUpdate\\":\\"$NOW\\"}" > "$STATUS_FILE"
    ;;
esac
`;

    fs.writeFileSync(hookScript, script, { mode: 0o755 });
  }

  private async configureClaudeHooks(sessionId: string, cwd: string, hookScript: string): Promise<void> {
    const config = vscode.workspace.getConfiguration('claudeAgents');
    if (!config.get('autoConfigureHooks', true)) return;

    // Write hooks to project-level .claude/settings.local.json (gitignored)
    const claudeDir = path.join(cwd, '.claude');
    if (!fs.existsSync(claudeDir)) {
      fs.mkdirSync(claudeDir, { recursive: true });
    }

    const settingsFile = path.join(claudeDir, 'settings.local.json');
    let settings: any = {};

    // Read existing settings if present
    if (fs.existsSync(settingsFile)) {
      try {
        settings = JSON.parse(fs.readFileSync(settingsFile, 'utf-8'));
      } catch {
        settings = {};
      }
    }

    // Add our hooks (don't overwrite existing ones)
    // Claude Code hooks schema:
    // { hooks: { EventName: [{ matcher: "...", hooks: [{ type: "command", command: "..." }] }] } }
    if (!settings.hooks) settings.hooks = {};

    const hookCommand = `bash "${hookScript}"`;

    for (const event of ['PostToolUse', 'Stop', 'Notification', 'UserPromptSubmit']) {
      if (!settings.hooks[event]) {
        settings.hooks[event] = [];
      }
      // Check if our hook already exists
      const existing = settings.hooks[event].find(
        (h: any) => h.hooks?.some((hh: any) => hh.command?.includes('.claude-agents'))
      );
      if (!existing) {
        settings.hooks[event].push({
          matcher: '',
          hooks: [
            {
              type: 'command',
              command: `${hookCommand} ${event}`,
            },
          ],
        });
      }
    }

    fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
  }

  private async detectBranch(cwd: string): Promise<string> {
    try {
      const { execSync } = require('child_process');
      const branch = execSync('git rev-parse --abbrev-ref HEAD', {
        cwd,
        encoding: 'utf-8',
        timeout: 3000,
      }).trim();
      return branch;
    } catch {
      return 'n/a';
    }
  }

  private startBranchPolling(id: string, cwd: string): void {
    const interval = setInterval(async () => {
      const session = this.sessions.get(id);
      if (!session) {
        this.stopBranchPolling(id);
        return;
      }
      const branch = await this.detectBranch(cwd);
      if (branch !== session.branch) {
        session.branch = branch;
        this.emitChange();
      }
    }, 10000);
    this.branchIntervals.set(id, interval);
  }

  private stopBranchPolling(id: string): void {
    const interval = this.branchIntervals.get(id);
    if (interval) {
      clearInterval(interval);
      this.branchIntervals.delete(id);
    }
  }

  private stopWatcher(id: string): void {
    const watcher = this.watchers.get(id);
    if (watcher) {
      watcher.close();
      this.watchers.delete(id);
    }
  }

  private cleanup(id: string): void {
    this.stopBranchPolling(id);
    this.stopWatcher(id);
    this.terminals.delete(id);
    this.sessions.delete(id);
    this.emitChange();
  }

  private emitChange(): void {
    this._onDidChangeSessions.fire(this.getSessions());
  }

  private shellEscape(str: string): string {
    return `'${str.replace(/'/g, "'\\''")}'`;
  }

  dispose(): void {
    for (const id of this.sessions.keys()) {
      this.stopBranchPolling(id);
      this.stopWatcher(id);
    }
    this._onDidChangeSessions.dispose();
  }
}
