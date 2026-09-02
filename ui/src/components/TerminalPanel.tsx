import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import type { SessionInfo } from "../lib/types";

interface TerminalPanelProps {
  session: SessionInfo;
  port: number;
  isActive: boolean;
}

type ConnectionState = "connecting" | "connected" | "disconnected" | "error";

export function TerminalPanel({ session, port, isActive }: TerminalPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const offsetRef = useRef<number>(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptRef = useRef<number>(0);
  const [connState, setConnState] = useState<ConnectionState>("connecting");

  // Initialize terminal once — stable across renders
  useEffect(() => {
    if (!containerRef.current || termRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      theme: {
        background: "#0d0d0d",
        foreground: "#e8e8d0",
        cursor: "#c8b400",
        selectionBackground: "#6b620080",
        black: "#1a1a0e",
        red: "#c83232",
        green: "#7ab800",
        yellow: "#d4c800",
        blue: "#8ab4f8",
        magenta: "#c8a0e0",
        cyan: "#80cbc4",
        white: "#e8e8d0",
      },
      fontFamily: "JetBrains Mono, Fira Code, Menlo, monospace",
      fontSize: 13,
      lineHeight: 1.2,
      scrollback: 5000,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);

    requestAnimationFrame(() => {
      try { fitAddon.fit(); } catch { /* container not visible yet */ }
    });

    termRef.current = term;
    fitRef.current = fitAddon;

    return () => {
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
  }, []);

  // WebSocket connection with auto-reconnect — only depends on session.id and port
  useEffect(() => {
    let disposed = false;

    const connectWs = () => {
      if (disposed) return;
      const term = termRef.current;
      if (!term) return;

      setConnState("connecting");
      const ws = new WebSocket(`ws://127.0.0.1:${port}/ws/session/${session.id}`);
      ws.binaryType = "arraybuffer";
      wsRef.current = ws;

      ws.onopen = () => {
        if (disposed) return;
        setConnState("connected");
        reconnectAttemptRef.current = 0;
        // Request scrollback replay from where we left off
        ws.send(JSON.stringify({
          type: "reconnect",
          session_id: session.id,
          last_byte_offset: offsetRef.current,
        }));
      };

      ws.onmessage = (event) => {
        if (disposed || !termRef.current) return;
        if (event.data instanceof ArrayBuffer) {
          const bytes = new Uint8Array(event.data);
          offsetRef.current += bytes.length;
          termRef.current.write(bytes);
        } else {
          try {
            const data = JSON.parse(event.data);
            if (data.type === "agent_exit") {
              termRef.current.writeln(`\r\n\x1b[31m[Agent exited: ${data.message}]\x1b[0m`);
              setConnState("disconnected");
            } else if (data.type === "error") {
              termRef.current.writeln(`\r\n\x1b[31m[Error: ${data.message}]\x1b[0m`);
            }
          } catch {
            // Ignore unparseable text frames
          }
        }
      };

      ws.onclose = () => {
        if (disposed) return;
        setConnState("disconnected");
        wsRef.current = null;
        // Exponential backoff reconnect: 1s, 2s, 4s, 8s, max 30s
        const delay = Math.min(1000 * Math.pow(2, reconnectAttemptRef.current), 30000);
        reconnectAttemptRef.current++;
        if (termRef.current) {
          termRef.current.writeln(`\r\n\x1b[90m[Reconnecting in ${delay / 1000}s...]\x1b[0m`);
        }
        reconnectTimerRef.current = setTimeout(connectWs, delay);
      };

      ws.onerror = () => {
        if (disposed) return;
        setConnState("error");
      };

      // Keyboard input → PTY
      const dataDisposable = term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
          const encoder = new TextEncoder();
          ws.send(encoder.encode(data));
        }
      });

      // Store disposable for cleanup
      return dataDisposable;
    };

    const dataDisposable = connectWs();

    return () => {
      disposed = true;
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      dataDisposable?.dispose();
    };
  }, [session.id, port]);

  // Resize observer — separate effect, doesn't recreate terminal or WS
  useEffect(() => {
    if (!containerRef.current) return;

    const resizeObserver = new ResizeObserver(() => {
      if (!isActive || !fitRef.current || !wsRef.current) return;
      try {
        fitRef.current.fit();
        if (wsRef.current.readyState === WebSocket.OPEN && termRef.current) {
          wsRef.current.send(JSON.stringify({
            type: "resize",
            session_id: session.id,
            rows: termRef.current.rows,
            cols: termRef.current.cols,
          }));
        }
      } catch { /* ignore */ }
    });
    resizeObserver.observe(containerRef.current);

    return () => resizeObserver.disconnect();
  }, [isActive, session.id]);

  // Re-fit when panel becomes active
  useEffect(() => {
    if (isActive && fitRef.current) {
      requestAnimationFrame(() => {
        try { fitRef.current!.fit(); } catch { /* ignore */ }
      });
    }
  }, [isActive]);

  const statusColor = connState === "connected"
    ? "bg-status-success"
    : connState === "connecting"
    ? "bg-yellow-500"
    : connState === "error"
    ? "bg-status-error"
    : "bg-status-idle";

  const statusLabel = connState === "connected"
    ? session.agent_binary || session.provider
    : connState === "connecting"
    ? "connecting..."
    : connState === "error"
    ? "error"
    : "disconnected";

  return (
    <div className="h-full flex flex-col bg-[var(--panel-bg)] rounded-lg border border-[var(--border-color)] overflow-hidden">
      {/* Panel header */}
      <div className="h-8 flex items-center px-3 gap-2 bg-[var(--panel-header-bg)] border-b border-[var(--border-color)] shrink-0">
        <span className={`w-2 h-2 rounded-full ${statusColor} ${connState === "connected" ? "status-pulse" : ""}`} />
        <span className="text-xs font-medium text-gray-300 truncate">
          {statusLabel}
        </span>
        {session.workspace_path && (
          <span className="text-[10px] text-gray-500 bg-surface px-1.5 py-0.5 rounded truncate max-w-[120px]">
            ~/{session.workspace_path.split(/[/\\]/).pop()}
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