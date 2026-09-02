import { useState } from "react";
import { TerminalPanel } from "./TerminalPanel";
import type { SessionInfo, LayoutMode } from "../lib/types";

interface AgentInfo {
  name: string;
  binary: string;
  status: string;
}

interface PanelGridProps {
  sessions: SessionInfo[];
  layout: LayoutMode;
  port: number;
  agents: AgentInfo[];
  onSpawnSession: (agentBinary: string, accountId?: string) => void;
}

export function PanelGrid({ sessions, layout, port, agents, onSpawnSession }: PanelGridProps) {
  const [focusedId, setFocusedId] = useState<string | null>(null);

  if (sessions.length === 0) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center max-w-md">
          <div className="text-gray-400 text-sm mb-4">Nenhuma sessão ativa</div>
          <div className="grid grid-cols-2 gap-3">
            {agents
              .filter((a) => a.status !== "not_found")
              .map((agent) => (
                <button
                  key={agent.binary}
                  onClick={() => onSpawnSession(agent.binary)}
                  className="px-4 py-3 bg-surface-raised border border-[var(--border-color)] rounded-lg text-sm text-gray-300 hover:border-accent/50 hover:text-accent transition-colors"
                >
                  <div className="font-medium">{agent.name}</div>
                  <div className="text-xs text-gray-500 mt-1">{agent.binary}</div>
                </button>
              ))}
          </div>
          {agents.filter((a) => a.status !== "not_found").length === 0 && (
            <div className="text-gray-600 text-xs mt-2">
              Nenhum agente disponível. Instale claude, codex ou outro agent CLI.
            </div>
          )}
        </div>
      </div>
    );
  }

  // Grid layout: equal panels in a responsive grid
  if (layout === "grid") {
    const cols = sessions.length <= 2 ? 2 : sessions.length <= 4 ? 2 : 3;
    return (
      <div
        className="h-full grid gap-[var(--panel-gap)]"
        style={{
          gridTemplateColumns: `repeat(${cols}, 1fr)`,
          gridTemplateRows: `repeat(${Math.ceil(sessions.length / cols)}, 1fr)`,
        }}
      >
        {sessions.map((session) => (
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
      ? sessions.find((s) => s.id === focusedId) || sessions[0]
      : sessions[0];
    const others = sessions.filter((s) => s.id !== spotlight.id);

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
      {sessions.map((session) => (
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