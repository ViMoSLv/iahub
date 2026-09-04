import { useState, useRef, useEffect, useCallback } from "react";
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
  onReorderSessions?: (orderedIds: string[]) => void;
  onTerminate?: (sessionId: string) => void;
}

export function PanelGrid({ sessions, layout, port, agents, onSpawnSession, onReorderSessions, onTerminate }: PanelGridProps) {
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [orderedIds, setOrderedIds] = useState<string[]>(sessions.map(s => s.id));
  const [ctrlHeld, setCtrlHeld] = useState(false);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);
  const dragNodeRef = useRef<HTMLElement | null>(null);

  // Keep orderedIds in sync when sessions change
  useEffect(() => {
    const currentIds = new Set(sessions.map(s => s.id));
    setOrderedIds(prev => {
      const filtered = prev.filter(id => currentIds.has(id));
      const existingSet = new Set(filtered);
      for (const s of sessions) {
        if (!existingSet.has(s.id)) {
          filtered.push(s.id);
        }
      }
      return filtered;
    });
  }, [sessions]);

  // Track Ctrl key globally
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Control") setCtrlHeld(true);
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key === "Control") {
        setCtrlHeld(false);
        setDraggingId(null);
        setDropTargetId(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, []);

  const orderedSessions = orderedIds
    .map(id => sessions.find(s => s.id === id))
    .filter(Boolean) as SessionInfo[];

  const handleDragStart = useCallback((e: React.DragEvent, sessionId: string) => {
    if (!ctrlHeld) {
      e.preventDefault();
      return;
    }
    setDraggingId(sessionId);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", sessionId);
    if (e.currentTarget instanceof HTMLElement) {
      dragNodeRef.current = e.currentTarget;
      requestAnimationFrame(() => {
        if (dragNodeRef.current) {
          dragNodeRef.current.style.opacity = "0.4";
        }
      });
    }
  }, [ctrlHeld]);

  const handleDragEnd = useCallback(() => {
    if (dragNodeRef.current) {
      dragNodeRef.current.style.opacity = "1";
      dragNodeRef.current = null;
    }
    setDraggingId(null);
    setDropTargetId(null);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, targetId: string) => {
    if (!ctrlHeld || !draggingId || draggingId === targetId) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    setDropTargetId(targetId);
  }, [ctrlHeld, draggingId]);

  const handleDragLeave = useCallback(() => {
    setDropTargetId(null);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent, targetId: string) => {
    e.preventDefault();
    if (!draggingId || draggingId === targetId) return;
    setOrderedIds(prev => {
      const newOrder = [...prev];
      const fromIdx = newOrder.indexOf(draggingId);
      const toIdx = newOrder.indexOf(targetId);
      if (fromIdx === -1 || toIdx === -1) return prev;
      newOrder.splice(fromIdx, 1);
      newOrder.splice(toIdx, 0, draggingId);
      return newOrder;
    });
    onReorderSessions?.(orderedIds);
    setDraggingId(null);
    setDropTargetId(null);
    if (dragNodeRef.current) {
      dragNodeRef.current.style.opacity = "1";
      dragNodeRef.current = null;
    }
  }, [draggingId, onReorderSessions, orderedIds]);

  if (sessions.length === 0) {
    return (
      <div className="h-full flex items-center justify-center bg-[#0B0B0B]">
        <div className="text-center max-w-md">
          <div className="w-16 h-16 rounded-2xl bg-[#121212] border border-[#232323] flex items-center justify-center mx-auto mb-4">
            <span className="text-2xl">🖥️</span>
          </div>
          <div className="text-[#A3A3A3] text-sm mb-1">Nenhuma sessão ativa</div>
          <div className="text-[#555] text-xs mb-5">Inicie uma sessão pelo botão "+ Nova Sessão" no header</div>
          <div className="grid grid-cols-2 gap-2">
            {agents
              .filter((a) => a.status !== "not_found")
              .map((agent) => (
                <button
                  key={agent.binary}
                  onClick={() => onSpawnSession(agent.binary)}
                  className="px-3 py-2.5 bg-[#121212] border border-[#232323] rounded-lg text-sm text-[#C9C9C9] hover:border-[#007acc]/40 hover:bg-[#1A1A1A] transition-colors group"
                >
                  <div className="flex items-center gap-2">
                    <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                      agent.status === "ok" ? "bg-[#4ADE80]" : "bg-[#F0C24B]"
                    }`} />
                    <span className="font-medium text-[12px] group-hover:text-[#DCDCDC]">{agent.name}</span>
                  </div>
                  <div className="text-[10px] text-[#555] mt-0.5 ml-3.5">{agent.binary}</div>
                </button>
              ))}
          </div>
          {agents.filter((a) => a.status !== "not_found").length === 0 && (
            <div className="text-[#555] text-[11px] mt-3">
              Nenhum agente disponível. Instale claude, codex ou outro agent CLI.
            </div>
          )}
        </div>
      </div>
    );
  }

  const panelWrapperClass = (sessionId: string) =>
    `min-h-0 min-w-0 transition-all duration-150 rounded-lg overflow-hidden ${
      ctrlHeld ? "cursor-grab active:cursor-grabbing" : ""
    } ${
      dropTargetId === sessionId && draggingId !== sessionId
        ? "ring-2 ring-[#007acc] scale-[1.01]"
        : ""
    } ${
      draggingId === sessionId ? "opacity-40" : ""
    }`;

  // Grid layout
  if (layout === "grid") {
    const cols = orderedSessions.length <= 2 ? 2 : orderedSessions.length <= 4 ? 2 : 3;
    return (
      <div
        className="h-full grid gap-1"
        style={{
          gridTemplateColumns: `repeat(${cols}, 1fr)`,
          gridTemplateRows: `repeat(${Math.ceil(orderedSessions.length / cols)}, 1fr)`,
        }}
      >
        {orderedSessions.map((session) => (
          <div
            key={session.id}
            className={panelWrapperClass(session.id)}
            draggable={ctrlHeld}
            onDragStart={(e) => handleDragStart(e, session.id)}
            onDragEnd={handleDragEnd}
            onDragOver={(e) => handleDragOver(e, session.id)}
            onDragLeave={handleDragLeave}
            onDrop={(e) => handleDrop(e, session.id)}
            onClick={() => setFocusedId(session.id)}
          >
            <TerminalPanel
              session={session}
              port={port}
              isActive={focusedId === session.id || focusedId === null}
              onTerminate={onTerminate}
            />
          </div>
        ))}
      </div>
    );
  }

  // Spotlight layout
  if (layout === "spotlight") {
    const spotlight = focusedId
      ? orderedSessions.find((s) => s.id === focusedId) || orderedSessions[0]
      : orderedSessions[0];
    const others = orderedSessions.filter((s) => s.id !== spotlight.id);
    return (
      <div className="h-full flex gap-1">
        <div
          className={`flex-1 min-w-0 min-h-0 transition-all duration-150 rounded-lg overflow-hidden ${
            ctrlHeld ? "cursor-grab active:cursor-grabbing" : ""
          } ${
            dropTargetId === spotlight.id && draggingId !== spotlight.id
              ? "ring-2 ring-[#007acc] scale-[1.01]"
              : ""
          } ${draggingId === spotlight.id ? "opacity-40" : ""}`}
          draggable={ctrlHeld}
          onDragStart={(e) => handleDragStart(e, spotlight.id)}
          onDragEnd={handleDragEnd}
          onDragOver={(e) => handleDragOver(e, spotlight.id)}
          onDragLeave={handleDragLeave}
          onDrop={(e) => handleDrop(e, spotlight.id)}
          onClick={() => setFocusedId(spotlight.id)}
        >
          <TerminalPanel session={spotlight} port={port} isActive={true} onTerminate={onTerminate} />
        </div>
        {others.length > 0 && (
          <div className="w-[35%] flex flex-col gap-1 min-h-0">
            {others.map((session) => (
              <div
                key={session.id}
                className={`flex-1 min-h-0 transition-all duration-150 rounded-lg overflow-hidden ${
                  ctrlHeld ? "cursor-grab active:cursor-grabbing" : ""
                } ${
                  dropTargetId === session.id && draggingId !== session.id
                    ? "ring-2 ring-[#007acc] scale-[1.01]"
                    : ""
                } ${draggingId === session.id ? "opacity-40" : ""}`}
                draggable={ctrlHeld}
                onDragStart={(e) => handleDragStart(e, session.id)}
                onDragEnd={handleDragEnd}
                onDragOver={(e) => handleDragOver(e, session.id)}
                onDragLeave={handleDragLeave}
                onDrop={(e) => handleDrop(e, session.id)}
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

  // Sidebar layout (vertical stack)
  return (
    <div className="h-full flex flex-col gap-1">
      {orderedSessions.map((session) => (
        <div
          key={session.id}
          className={`flex-1 min-h-0 transition-all duration-150 rounded-lg overflow-hidden ${
            ctrlHeld ? "cursor-grab active:cursor-grabbing" : ""
          } ${
            dropTargetId === session.id && draggingId !== session.id
              ? "ring-2 ring-[#007acc] scale-[1.01]"
              : ""
          } ${draggingId === session.id ? "opacity-40" : ""}`}
          draggable={ctrlHeld}
          onDragStart={(e) => handleDragStart(e, session.id)}
          onDragEnd={handleDragEnd}
          onDragOver={(e) => handleDragOver(e, session.id)}
          onDragLeave={handleDragLeave}
          onDrop={(e) => handleDrop(e, session.id)}
          onClick={() => setFocusedId(session.id)}
        >
          <TerminalPanel
            session={session}
            port={port}
            isActive={focusedId === session.id || focusedId === null}
            onTerminate={onTerminate}
          />
        </div>
      ))}
    </div>
  );
}