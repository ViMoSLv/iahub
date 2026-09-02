import { useEffect, useRef, useCallback } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import type { SessionInfo } from "../lib/types";

interface TerminalPanelProps {
  session: SessionInfo;
  port: number;
  isActive: boolean;
}

export function TerminalPanel({ session, port, isActive }: TerminalPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const offsetRef = useRef<number>(0);

  const connect = useCallback(() => {
    if (!containerRef.current) return;

    // Initialize xterm.js
    const term = new Terminal({
      cursorBlink: true,
      theme: {
        background: "#12121f",
        foreground: "#e0e0e0",
        cursor: "#e94560",
        selectionBackground: "#53348380",
        black: "#1a1a2e",
        red: "#e94560",
        green: "#0ead69",
        yellow: "#f0c040",
        blue: "#00d2ff",
        magenta: "#c77dff",
        cyan: "#48cae4",
        white: "#e0e0e0",
      },
      fontFamily: "JetBrains Mono, Fira Code, Menlo, monospace",
      fontSize: 13,
      lineHeight: 1.2,
      scrollback: 5000,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);

    // Delay fit to ensure DOM is ready
    requestAnimationFrame(() => {
      try { fitAddon.fit(); } catch { /* container not visible yet */ }
    });

    termRef.current = term;
    fitRef.current = fitAddon;

    // Connect WebSocket
    const ws = new WebSocket(`ws://127.0.0.1:${port}/ws/session/${session.id}`);
    ws.binaryType = "arraybuffer";
    wsRef.current = ws;

    ws.onopen = () => {
      // Send reconnect with current offset for scrollback replay
      const msg = JSON.stringify({
        type: "reconnect",
        session_id: session.id,
        last_byte_offset: offsetRef.current,
      });
      ws.send(msg);
    };

    ws.onmessage = (event) => {
      if (event.data instanceof ArrayBuffer) {
        // Binary frame: terminal output
        const bytes = new Uint8Array(event.data);
        offsetRef.current += bytes.length;
        term.write(bytes);
      } else {
        // Text frame: control event from backend
        try {
          const data = JSON.parse(event.data);
          if (data.type === "agent_exit") {
            term.writeln(`\r\n\x1b[31m[Agent exited: ${data.message}]\x1b[0m`);
          } else if (data.type === "error") {
            term.writeln(`\r\n\x1b[31m[Error: ${data.message}]\x1b[0m`);
          }
        } catch {
          // Ignore unparseable text frames
        }
      }
    };

    ws.onclose = () => {
      term.writeln("\r\n\x1b[90m[Connection closed]\x1b[0m");
    };

    ws.onerror = () => {
      term.writeln("\r\n\x1b[31m[Connection error]\x1b[0m");
    };

    // Keyboard input → PTY
    term.onData((data) => {
      if (ws.readyState === WebSocket.OPEN) {
        const encoder = new TextEncoder();
        ws.send(encoder.encode(data));
      }
    });

    // Resize observer
    const resizeObserver = new ResizeObserver(() => {
      if (!isActive) return;
      try {
        fitAddon.fit();
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({
            type: "resize",
            session_id: session.id,
            rows: term.rows,
            cols: term.cols,
          }));
        }
      } catch { /* ignore */ }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      resizeObserver.disconnect();
      ws.close();
      term.dispose();
    };
  }, [session.id, port, isActive]);

  useEffect(() => {
    const cleanup = connect();
    return cleanup;
  }, [connect]);

  // Re-fit when panel becomes active
  useEffect(() => {
    if (isActive && fitRef.current) {
      requestAnimationFrame(() => {
        try { fitRef.current!.fit(); } catch { /* ignore */ }
      });
    }
  }, [isActive]);

  const statusColor = session.status === "active"
    ? "bg-status-success"
    : session.status === "idle"
    ? "bg-yellow-500"
    : "bg-status-idle";

  return (
    <div className="h-full flex flex-col bg-[var(--panel-bg)] rounded-lg border border-[var(--border-color)] overflow-hidden">
      {/* Panel header */}
      <div className="h-8 flex items-center px-3 gap-2 bg-[var(--panel-header-bg)] border-b border-[var(--border-color)] shrink-0">
        <span className={`w-2 h-2 rounded-full ${statusColor} ${session.status === "active" ? "status-pulse" : ""}`} />
        <span className="text-xs font-medium text-gray-300 truncate">
          {session.agent_binary || session.provider}
        </span>
        {session.workspace_path && (
          <span className="text-[10px] text-gray-500 bg-surface px-1.5 py-0.5 rounded truncate max-w-[120px]">
            ~/{session.workspace_path.split("/").pop()}
          </span>
        )}
        <div className="flex-1" />
        {/* Action buttons */}
        <button
          className="text-gray-500 hover:text-gray-300 text-xs px-1"
          title="Interrupt (Ctrl+C)"
          onClick={() => {
            if (wsRef.current?.readyState === WebSocket.OPEN) {
              wsRef.current.send(JSON.stringify({
                type: "interrupt",
                session_id: session.id,
              }));
            }
          }}
        >
          ⏹
        </button>
        <button className="text-gray-500 hover:text-gray-300 text-xs px-1" title="Expand">
          ⛶
        </button>
      </div>
      {/* Terminal area */}
      <div ref={containerRef} className="flex-1 min-h-0" />
    </div>
  );
}