import { Group, Panel } from "react-resizable-panels";
import { TerminalPanel } from "./TerminalPanel";
import type { SessionInfo } from "../lib/types";

interface ResizablePanelGridProps {
  sessions: SessionInfo[];
  port: number;
  onTerminate?: (sessionId: string) => void;
}

export function ResizablePanelGrid({ sessions, port, onTerminate }: ResizablePanelGridProps) {
  if (sessions.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-[#555] text-sm">
        Nenhuma sessão ativa — use + Nova Sessão ou Ctrl+K
      </div>
    );
  }

  if (sessions.length === 1) {
    return (
      <div className="h-full">
        <TerminalPanel session={sessions[0]} port={port} isActive={true} />
      </div>
    );
  }

  if (sessions.length === 2) {
    return (
      <Group orientation="horizontal" className="h-full">
        <Panel defaultSize={50} minSize={20}>
          <TerminalPanel session={sessions[0]} port={port} isActive={true} />
        </Panel>
        <Panel defaultSize={50} minSize={20}>
          <TerminalPanel session={sessions[1]} port={port} isActive={true} />
        </Panel>
      </Group>
    );
  }

  if (sessions.length === 3) {
    return (
      <Group orientation="horizontal" className="h-full">
        <Panel defaultSize={50} minSize={20}>
          <TerminalPanel session={sessions[0]} port={port} isActive={true} />
        </Panel>
        <Panel defaultSize={50} minSize={20}>
          <Group orientation="vertical" className="h-full">
            <Panel defaultSize={50} minSize={20}>
              <TerminalPanel session={sessions[1]} port={port} isActive={true} />
            </Panel>
            <Panel defaultSize={50} minSize={20}>
              <TerminalPanel session={sessions[2]} port={port} isActive={true} />
            </Panel>
          </Group>
        </Panel>
      </Group>
    );
  }

  // 4+ sessions: 2-column layout with resizable panels
  const leftSessions = sessions.slice(0, Math.ceil(sessions.length / 2));
  const rightSessions = sessions.slice(Math.ceil(sessions.length / 2));

  return (
    <Group orientation="horizontal" className="h-full">
      <Panel defaultSize={50} minSize={20}>
        <Group orientation="vertical" className="h-full">
          {leftSessions.map((session) => (
            <Panel key={session.id} defaultSize={100 / leftSessions.length} minSize={15}>
              <TerminalPanel session={session} port={port} isActive={true} onTerminate={onTerminate} />
            </Panel>
          ))}
        </Group>
      </Panel>
      <Panel defaultSize={50} minSize={20}>
        <Group orientation="vertical" className="h-full">
          {rightSessions.map((session) => (
            <Panel key={session.id} defaultSize={100 / rightSessions.length} minSize={15}>
              <TerminalPanel session={session} port={port} isActive={true} onTerminate={onTerminate} />
            </Panel>
          ))}
        </Group>
      </Panel>
    </Group>
  );
}