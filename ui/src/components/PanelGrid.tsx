import { useState } from "react";
import { TerminalPanel } from "./TerminalPanel";
import type { SessionInfo, LayoutMode } from "../lib/types";

interface PanelGridProps {
  sessions: SessionInfo[];
  layout: LayoutMode;
  port: number;
}

// Demo sessions for visual development before backend is fully wired
const DEMO_SESSIONS: SessionInfo[] = [
  {
    id: "demo-claude-1",
    account_id: "claude-a",
    provider: "claude",
    agent_binary: "claude",
    status: "active",
    task_description: "refactor auth",
    workspace_path: "/home/user/barbearia",
  },
  {
    id: "demo-codex-1",
    account_id: "codex-a",
    provider: "codex",
    agent_binary: "codex",
    status: "active",
    task_description: "gen types",
    workspace_path: "/home/user/loja-online",
  },
  {
    id: "demo-claude-2",
    account_id: "claude-b",
    provider: "claude",
    agent_binary: "claude",
    status: "idle",
    task_description: "fix lint",
    workspace_path: "/home/user/discord-bot",
  },
  {
    id: "demo-shell-1",
    account_id: "shell-local",
    provider: "shell",
    agent_binary: "shell",
    status: "idle",
    task_description: "pnpm build",
    workspace_path: "/home/user/blog",
  },
];

export function PanelGrid({ sessions, layout, port }: PanelGridProps) {
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const displaySessions = sessions.length > 0 ? sessions : DEMO_SESSIONS;

  if (displaySessions.length === 0) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="text-gray-500 text-sm mb-2">Nenhuma sessão ativa</div>
          <button className="text-accent text-sm hover:underline">
            + Iniciar nova sessão
          </button>
        </div>
      </div>
    );
  }

  // Grid layout: equal panels in a responsive grid
  if (layout === "grid") {
    const cols = displaySessions.length <= 2 ? 2 : displaySessions.length <= 4 ? 2 : 3;
    return (
      <div
        className="h-full grid gap-[var(--panel-gap)]"
        style={{
          gridTemplateColumns: `repeat(${cols}, 1fr)`,
          gridTemplateRows: `repeat(${Math.ceil(displaySessions.length / cols)}, 1fr)`,
        }}
      >
        {displaySessions.map((session) => (
          <div
            key={session.id}
            className="min-h-0 min-w-0"
            onClick={() => setFocusedId(session.id)}
          >
            <TerminalPanel
              session={session}
              port={port}
              isActive={focusedId === session.id || focusedId === null}
            />
          </div>
        ))}
      </div>
    );
  }

  // Spotlight layout: one large panel + smaller ones on the side
  if (layout === "spotlight") {
    const spotlight = focusedId
      ? displaySessions.find((s) => s.id === focusedId) || displaySessions[0]
      : displaySessions[0];
    const others = displaySessions.filter((s) => s.id !== spotlight.id);

    return (
      <div className="h-full flex gap-[var(--panel-gap)]">
        {/* Main spotlight panel */}
        <div className="flex-1 min-w-0 min-h-0" onClick={() => setFocusedId(spotlight.id)}>
          <TerminalPanel session={spotlight} port={port} isActive={true} />
        </div>
        {/* Side panels */}
        {others.length > 0 && (
          <div className="w-[35%] flex flex-col gap-[var(--panel-gap)] min-h-0">
            {others.map((session) => (
              <div
                key={session.id}
                className="flex-1 min-h-0"
                onClick={() => setFocusedId(session.id)}
              >
                <TerminalPanel session={session} port={port} isActive={false} />
              </div>
            ))}
          </div>
        )}
      </div>
    );
  }

  // Sidebar layout: vertical stack of panels
  return (
    <div className="h-full flex flex-col gap-[var(--panel-gap)]">
      {displaySessions.map((session) => (
        <div
          key={session.id}
          className="flex-1 min-h-0"
          onClick={() => setFocusedId(session.id)}
        >
          <TerminalPanel
            session={session}
            port={port}
            isActive={focusedId === session.id || focusedId === null}
          />
        </div>
      ))}
    </div>
  );
}