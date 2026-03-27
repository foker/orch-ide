import * as vscode from 'vscode';
import { ISession, WebviewMessage, SessionStatus } from './types';
import { SessionManager } from './sessionManager';

export class CardPanelProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'claudeAgents.cardPanel';

  private view?: vscode.WebviewView;

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly sessionManager: SessionManager
  ) {
    this.sessionManager.onDidChangeSessions((sessions) => {
      this.updateWebview(sessions);
    });
  }

  resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ): void {
    this.view = webviewView;

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [this.extensionUri],
    };

    webviewView.webview.html = this.getHtml();

    webviewView.webview.onDidReceiveMessage((message: WebviewMessage) => {
      this.handleMessage(message);
    });

    // Send initial state
    this.updateWebview(this.sessionManager.getSessions());
  }

  private async handleMessage(message: WebviewMessage): Promise<void> {
    switch (message.type) {
      case 'newSession':
        await vscode.commands.executeCommand('claudeAgents.newSession');
        break;
      case 'killSession':
        if (message.sessionId) {
          this.sessionManager.killSession(message.sessionId);
        }
        break;
      case 'focusSession':
        if (message.sessionId) {
          this.sessionManager.focusSession(message.sessionId);
        }
        break;
      case 'refresh':
        this.updateWebview(this.sessionManager.getSessions());
        break;
    }
  }

  private updateWebview(sessions: ISession[]): void {
    if (this.view) {
      this.view.webview.postMessage({
        type: 'sessionsUpdated',
        sessions,
      });
    }
  }

  private getHtml(): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }

    body {
      font-family: var(--vscode-font-family, -apple-system, BlinkMacSystemFont, sans-serif);
      font-size: var(--vscode-font-size, 13px);
      color: var(--vscode-foreground);
      background: var(--vscode-sideBar-background, transparent);
      padding: 8px;
    }

    .header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 8px;
      padding-bottom: 8px;
      border-bottom: 1px solid var(--vscode-widget-border, rgba(255,255,255,0.1));
    }

    .header-title {
      font-weight: 600;
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: var(--vscode-sideBarSectionHeader-foreground, var(--vscode-foreground));
    }

    .overall-progress {
      width: 100%;
      height: 3px;
      background: var(--vscode-progressBar-background, rgba(255,255,255,0.1));
      border-radius: 2px;
      margin-top: 6px;
      overflow: hidden;
    }

    .overall-progress-fill {
      height: 100%;
      background: var(--vscode-progressBar-background, #0078d4);
      border-radius: 2px;
      transition: width 0.3s ease;
    }

    .btn-add {
      background: none;
      border: none;
      color: var(--vscode-foreground);
      cursor: pointer;
      font-size: 16px;
      padding: 2px 6px;
      border-radius: 3px;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .btn-add:hover {
      background: var(--vscode-toolbar-hoverBackground, rgba(255,255,255,0.1));
    }

    .sessions-list {
      display: flex;
      flex-direction: column;
      gap: 6px;
    }

    .session-card {
      background: var(--vscode-editor-background, #1e1e1e);
      border: 1px solid var(--vscode-widget-border, rgba(255,255,255,0.08));
      border-radius: 6px;
      padding: 10px 12px;
      cursor: pointer;
      transition: border-color 0.15s ease;
    }

    .session-card:hover {
      border-color: var(--vscode-focusBorder, #007fd4);
    }

    .card-top {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 6px;
    }

    .status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      flex-shrink: 0;
    }

    .status-dot.running { background: #4ec94e; }
    .status-dot.idle { background: #888; }
    .status-dot.awaiting-input { background: #e8a838; }
    .status-dot.done { background: #4ec94e; }
    .status-dot.error { background: #f44747; }

    .card-name {
      font-weight: 600;
      font-size: 13px;
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .card-branch {
      font-size: 11px;
      color: var(--vscode-descriptionForeground, #888);
      opacity: 0.8;
    }

    .card-task {
      font-size: 12px;
      color: var(--vscode-descriptionForeground, #aaa);
      margin-bottom: 6px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .progress-row {
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .progress-bar {
      flex: 1;
      height: 4px;
      background: rgba(255,255,255,0.1);
      border-radius: 2px;
      overflow: hidden;
    }

    .progress-fill {
      height: 100%;
      border-radius: 2px;
      transition: width 0.3s ease;
    }

    .progress-fill.running { background: #4ec94e; }
    .progress-fill.awaiting-input { background: #e8a838; }
    .progress-fill.done { background: #4ec94e; }
    .progress-fill.error { background: #f44747; }
    .progress-fill.idle { background: #888; }

    .progress-text {
      font-size: 11px;
      color: var(--vscode-descriptionForeground, #888);
      white-space: nowrap;
    }

    .card-status {
      font-size: 11px;
      margin-top: 4px;
      color: var(--vscode-descriptionForeground, #888);
    }

    .card-actions {
      display: flex;
      gap: 4px;
    }

    .btn-kill {
      background: none;
      border: none;
      color: var(--vscode-descriptionForeground, #888);
      cursor: pointer;
      font-size: 12px;
      padding: 1px 4px;
      border-radius: 3px;
      opacity: 0;
      transition: opacity 0.15s;
    }

    .session-card:hover .btn-kill {
      opacity: 1;
    }

    .btn-kill:hover {
      color: #f44747;
      background: rgba(244,71,71,0.1);
    }

    .empty-state {
      text-align: center;
      padding: 32px 16px;
      color: var(--vscode-descriptionForeground, #888);
    }

    .empty-state p {
      margin-bottom: 12px;
      font-size: 12px;
    }

    .empty-state button {
      background: var(--vscode-button-background, #0078d4);
      color: var(--vscode-button-foreground, #fff);
      border: none;
      padding: 6px 14px;
      border-radius: 3px;
      cursor: pointer;
      font-size: 12px;
    }

    .empty-state button:hover {
      background: var(--vscode-button-hoverBackground, #026ec1);
    }

    .status-label {
      font-size: 10px;
      padding: 1px 6px;
      border-radius: 3px;
      text-transform: uppercase;
      font-weight: 600;
      letter-spacing: 0.3px;
    }

    .status-label.running {
      background: rgba(78,201,78,0.15);
      color: #4ec94e;
    }

    .status-label.awaiting-input {
      background: rgba(232,168,56,0.15);
      color: #e8a838;
    }

    .status-label.done {
      background: rgba(78,201,78,0.15);
      color: #4ec94e;
    }

    .status-label.error {
      background: rgba(244,71,71,0.15);
      color: #f44747;
    }

    .status-label.idle {
      background: rgba(136,136,136,0.15);
      color: #888;
    }
  </style>
</head>
<body>
  <div class="header">
    <div>
      <span class="header-title">Claude Agents</span>
      <div class="overall-progress">
        <div class="overall-progress-fill" id="overallProgress" style="width: 0%"></div>
      </div>
    </div>
    <button class="btn-add" onclick="newSession()" title="New Session">+</button>
  </div>

  <div id="sessionsList" class="sessions-list">
    <div class="empty-state">
      <p>No active sessions</p>
      <button onclick="newSession()">Create Session</button>
    </div>
  </div>

  <script>
    const vscode = acquireVsCodeApi();

    function newSession() {
      vscode.postMessage({ type: 'newSession' });
    }

    function killSession(id, e) {
      e.stopPropagation();
      vscode.postMessage({ type: 'killSession', sessionId: id });
    }

    function focusSession(id) {
      vscode.postMessage({ type: 'focusSession', sessionId: id });
    }

    function getStatusLabel(status) {
      const labels = {
        'idle': 'Idle',
        'running': 'Running',
        'awaiting-input': 'Awaiting Input',
        'done': 'Done',
        'error': 'Error'
      };
      return labels[status] || status;
    }

    function renderSessions(sessions) {
      const list = document.getElementById('sessionsList');
      const overallBar = document.getElementById('overallProgress');

      if (!sessions || sessions.length === 0) {
        list.innerHTML = '<div class="empty-state"><p>No active sessions</p><button onclick="newSession()">Create Session</button></div>';
        overallBar.style.width = '0%';
        return;
      }

      // Calculate overall progress
      let totalSteps = 0;
      let completedSteps = 0;
      sessions.forEach(s => {
        if (s.progress && s.progress.totalSteps > 0) {
          totalSteps += s.progress.totalSteps;
          completedSteps += s.progress.step;
        }
      });
      const overallPct = totalSteps > 0 ? Math.round((completedSteps / totalSteps) * 100) : 0;
      overallBar.style.width = overallPct + '%';

      list.innerHTML = sessions.map(s => {
        const progress = s.progress;
        const pct = progress && progress.totalSteps > 0
          ? Math.round((progress.step / progress.totalSteps) * 100)
          : 0;
        const progressLabel = progress && progress.totalSteps > 0
          ? progress.step + '/' + progress.totalSteps
          : '';
        const etcLabel = progress && progress.etc ? 'ETC: ' + progress.etc : '';
        const taskText = progress && progress.currentTask ? progress.currentTask : '';

        return '<div class="session-card" onclick="focusSession(\\''+s.id+'\\')">' +
          '<div class="card-top">' +
            '<div class="status-dot ' + s.status + '"></div>' +
            '<span class="card-name">' + escapeHtml(s.name) + '</span>' +
            '<span class="card-branch">' + escapeHtml(s.branch) + '</span>' +
            '<span class="status-label ' + s.status + '">' + getStatusLabel(s.status) + '</span>' +
            '<div class="card-actions"><button class="btn-kill" onclick="killSession(\\''+s.id+'\\', event)" title="Kill session">✕</button></div>' +
          '</div>' +
          (taskText ? '<div class="card-task">' + escapeHtml(taskText) + '</div>' : '') +
          '<div class="progress-row">' +
            '<div class="progress-bar"><div class="progress-fill ' + s.status + '" style="width:' + pct + '%"></div></div>' +
            (progressLabel ? '<span class="progress-text">' + progressLabel + '</span>' : '') +
            (etcLabel ? '<span class="progress-text">' + etcLabel + '</span>' : '') +
          '</div>' +
          (s.status === 'awaiting-input' ? '<div class="card-status">Waiting for your input...</div>' : '') +
        '</div>';
      }).join('');
    }

    function escapeHtml(str) {
      const div = document.createElement('div');
      div.textContent = str;
      return div.innerHTML;
    }

    window.addEventListener('message', event => {
      const message = event.data;
      if (message.type === 'sessionsUpdated') {
        renderSessions(message.sessions);
      }
    });
  </script>
</body>
</html>`;
  }
}
