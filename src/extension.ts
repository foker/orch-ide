import * as vscode from 'vscode';
import { SessionManager } from './sessionManager';
import { CardPanelProvider } from './cardPanelProvider';

let sessionManager: SessionManager;

export function activate(context: vscode.ExtensionContext) {
  sessionManager = new SessionManager();

  // Register webview provider
  const cardPanelProvider = new CardPanelProvider(context.extensionUri, sessionManager);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(CardPanelProvider.viewType, cardPanelProvider)
  );

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('claudeAgents.newSession', async () => {
      const name = await vscode.window.showInputBox({
        prompt: 'Session name',
        placeHolder: 'e.g., fix-auth-bug',
      });
      if (!name) return;

      const folders = vscode.workspace.workspaceFolders;
      let cwd: string;

      if (folders && folders.length > 1) {
        const picked = await vscode.window.showWorkspaceFolderPick({
          placeHolder: 'Select working directory for this session',
        });
        cwd = picked?.uri.fsPath || folders[0].uri.fsPath;
      } else if (folders && folders.length === 1) {
        cwd = folders[0].uri.fsPath;
      } else {
        const uri = await vscode.window.showOpenDialog({
          canSelectFolders: true,
          canSelectFiles: false,
          canSelectMany: false,
          openLabel: 'Select Folder',
        });
        if (!uri || uri.length === 0) return;
        cwd = uri[0].fsPath;
      }

      const prompt = await vscode.window.showInputBox({
        prompt: 'Initial prompt for Claude (optional)',
        placeHolder: 'e.g., Fix the login bug in auth.ts',
      });

      try {
        await sessionManager.createSession(name, cwd, prompt || undefined);
      } catch (err: any) {
        vscode.window.showErrorMessage(`Failed to create session: ${err.message}`);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('claudeAgents.killSession', async () => {
      const sessions = sessionManager.getSessions();
      if (sessions.length === 0) {
        vscode.window.showInformationMessage('No active sessions');
        return;
      }

      const picked = await vscode.window.showQuickPick(
        sessions.map((s) => ({
          label: s.name,
          description: `${s.branch} — ${s.status}`,
          sessionId: s.id,
        })),
        { placeHolder: 'Select session to kill' }
      );

      if (picked) {
        sessionManager.killSession((picked as any).sessionId);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('claudeAgents.focusPanel', () => {
      vscode.commands.executeCommand('claudeAgents.cardPanel.focus');
    })
  );

  // Cleanup on deactivate
  context.subscriptions.push({
    dispose: () => sessionManager.dispose(),
  });
}

export function deactivate() {
  if (sessionManager) {
    sessionManager.dispose();
  }
}
