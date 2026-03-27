export type SessionStatus = 'idle' | 'running' | 'awaiting-input' | 'done' | 'error';

export interface SessionProgress {
  step: number;
  totalSteps: number;
  currentTask: string;
  etc?: string;
  lastUpdate: string;
}

export interface ISession {
  id: string;
  name: string;
  cwd: string;
  branch: string;
  status: SessionStatus;
  progress: SessionProgress | null;
  createdAt: string;
}

export interface StatusFilePayload {
  step?: number;
  totalSteps?: number;
  currentTask?: string;
  status?: SessionStatus;
  etc?: string;
  lastUpdate?: string;
}

export interface WebviewMessage {
  type: 'newSession' | 'killSession' | 'focusSession' | 'refresh';
  sessionId?: string;
}

export interface ExtensionMessage {
  type: 'sessionsUpdated';
  sessions: ISession[];
}
