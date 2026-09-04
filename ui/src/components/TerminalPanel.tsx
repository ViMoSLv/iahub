import { useEffect, useRef, useState, useCallback } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import type { SessionInfo } from "../lib/types";
import { TerminalSearch } from "./TerminalSearch";

interface TerminalPanelProps {
  session: SessionInfo;
  port: number;
  isActive: boolean;
  onTerminate?: (sessionId: string) => void;
}

type ConnectionState = "connecting" | "connected" | "disconnected" | "error";

export function TerminalPanel({ session, port, isActive, onTerminate }: TerminalPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const offsetRef = useRef<number>(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptRef = useRef<number>(0);
  const [connState, setConnState] = useState<ConnectionState>("connecting");
  const [searchVisible, setSearchVisible] = useState(false);

  const toggleSearch = useCallback(() => {
    setSearchVisible((v) => !v);
  }, []);

  // Ctrl+F to toggle search
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "f") {
        e.preventDefault();
        toggleSearch();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [toggleSearch]);

  // Initialize terminal once — stable across renders
  useEffect(() => {
    if (!containerRef.current || termRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      theme: {
        background: "#0B0B0B",
        foreground: "#DCDCDC",
        cursor: "#AEAFAD",
        cursorAccent: "#0B0B0B",
        selectionBackground: "#264F7880",
        selectionForeground: "#FFFFFF",
        black: "#000000",
        red: "#CD3131",
        green: "#0DBC79",
        yellow: "#E5E510",
        blue: "#2472C8",
        magenta: "#BC3FBC",
        cyan: "#11A8CD",
        white: "#E5E5E5",
        brightBlack: "#666666",
        brightRed: "#F14C4C",
        brightGreen: "#23D18B",
        brightYellow: "#F5F543",
        brightBlue: "#3B8EEA",
        brightMagenta: "#D670D6",
        brightCyan: "#29B8DB",
        brightWhite: "#FFFFFF",
      },
      fontFamily: "JetBrains Mono, Fira Code, Menlo, monospace",
      fontSize: 13,
      lineHeight: 1.2,
      scrollback: 5000,
      allowProposedApi: true,
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

      const dataDisposable = term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
          const encoder = new TextEncoder();
          ws.send(encoder.encode(data));
        }
      });

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

  // Resize observer — re-fit on container size changes
  useEffect(() => {
    if (!containerRef.current || !fitRef.current) return;
    const resizeObserver = new ResizeObserver(() => {
      try {
        fitRef.current?.fit();
        if (wsRef.current?.readyState === WebSocket.OPEN && termRef.current) {
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
    ? "bg-[#4ADE80]"
    : connState === "connecting"
    ? "bg-[#F0C24B]"
    : connState === "error"
    ? "bg-[#f44747]"
    : "bg-[#6E6E6E]";

  const statusLabel = connState === "connected"
    ? session.agent_binary || session.provider
    : connState === "connecting"
    ? "connecting..."
    : connState === "error"
    ? "error"
    : "disconnected";

  return (
    <div className="h-full flex flex-col bg-[#0B0B0B] rounded-lg border border-[#171717] overflow-hidden">
      {/* Panel header */}
      <div className="h-8 flex items-center px-3 gap-2 bg-[#121212] border-b border-[#171717] shrink-0">
        <span className={`w-2 h-2 rounded-full ${statusColor} ${connState === "connected" ? "status-pulse" : ""}`} />
        <span className="text-xs font-medium text-[#C9C9C9] truncate">
          {statusLabel}
        </span>
        {session.workspace_path && (
          <span className="text-[10px] text-[#7A7A7A] bg-[#0B0B0B] px-1.5 py-0.5 rounded truncate max-w-[120px] border border-[#232323]">
            ~/{session.workspace_path.split(/[/\\]/).pop()}
          </span>
        )}
        <div className="flex-1" />
        {/* Action buttons */}
        <button
          className="text-[#7A7A7A] hover:text-[#DCDCDC] text-xs px-1 transition-colors"
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
        <button className="text-[#7A7A7A] hover:text-[#DCDCDC] text-xs px-1 transition-colors" title="Expand">
          ⛶
        </button>
        {onTerminate && (
          <button
            className="text-[#7A7A7A] hover:text-[#f44747] text-xs px-1 transition-colors"
            title="Terminate session"
            onClick={() => onTerminate(session.id)}
          >
            ✕
          </button>
        )}
      </div>
      {/* Terminal area */}
      <div className="relative flex-1 min-h-0">
        <TerminalSearch
          terminal={termRef.current}
          visible={searchVisible}
          onClose={() => setSearchVisible(false)}
        />
        <div ref={containerRef} className="h-full" />
      </div>
    </div>
  );
}