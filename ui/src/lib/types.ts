// Shared types between frontend and backend API

export interface HealthResponse {
  status: "starting" | "ready" | "degraded" | "shutting_down";
  uptime_seconds: number;
  subsystems: {
    sqlite: string;
    pty_engine: string;
    credential_store: string;
  };
  active_sessions: number;
  schema_version: number;
  onboarding_required: boolean;
  agents: AgentBinaryStatus[];
}

export interface AgentBinaryStatus {
  name: string;
  path: string | null;
  version: string | null;
  status: "ok" | "not_found" | "version_unknown";
}

export interface SessionInfo {
  id: string;
  account_id: string;
  provider: string;
  agent_binary: string;
  status: "active" | "idle" | "exited";
  task_description?: string;
  workspace_path?: string;
}

export interface ProjectInfo {
  id: string;
  name: string;
  path: string;
  category: string; // "produtos", "pessoal", etc.
  session_count: number;
}

export interface ProviderAccountInfo {
  id: string;
  provider: string;
  label: string;
  status: "active" | "unavailable" | "rate_limited" | "authentication_required" | "disabled";
  max_concurrent_sessions: number;
  active_sessions: number;
}

export type LayoutMode = "grid" | "spotlight" | "sidebar";

// WebSocket control commands (sent as JSON text frames)
export interface ResizeCommand {
  type: "resize";
  session_id: string;
  rows: number;
  cols: number;
}

export interface InterruptCommand {
  type: "interrupt";
  session_id: string;
}

export interface ReconnectCommand {
  type: "reconnect";
  session_id: string;
  last_byte_offset: number;
}

export type ControlCommand = ResizeCommand | InterruptCommand | ReconnectCommand;

// WebSocket events from backend (received as JSON text frames)
export interface SessionStartedEvent {
  type: "session_started";
  session_id: string;
  account_id: string;
  agent_binary: string;
}

export interface AgentExitEvent {
  type: "agent_exit";
  session_id: string;
  exit_code: number | null;
  message: string;
}

export interface SpawnRejectedEvent {
  type: "spawn_rejected";
  account_id: string;
  max_concurrent: number;
  current_active: number;
  message: string;
}

export type SessionEvent = SessionStartedEvent | AgentExitEvent | SpawnRejectedEvent;