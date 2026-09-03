import { useRef, useEffect, useState } from "react";
import { Virtuoso } from "react-virtuoso";

export interface LogEntry {
  id: string;
  timestamp: string;
  level: "info" | "warn" | "error" | "debug";
  source: string;
  message: string;
}

interface LogViewerProps {
  logs: LogEntry[];
  autoScroll?: boolean;
  maxVisible?: number;
}

const levelColors: Record<LogEntry["level"], string> = {
  info: "text-[#48cae4]",
  warn: "text-[#f0c040]",
  error: "text-[#e94560]",
  debug: "text-[#7b8794]",
};

const levelBg: Record<LogEntry["level"], string> = {
  info: "bg-[#48cae410]",
  warn: "bg-[#f0c04010]",
  error: "bg-[#e9456010]",
  debug: "bg-transparent",
};

function LogRow({ entry }: { entry: LogEntry }) {
  return (
    <div
      className={`flex items-start gap-2 px-3 py-0.5 text-xs font-mono border-b border-[#1a1a2e] ${levelBg[entry.level]} hover:bg-white/[0.02]`}
    >
      <span className="text-[#555] shrink-0 w-[70px]">
        {entry.timestamp.slice(11, 19)}
      </span>
      <span
        className={`shrink-0 w-[40px] font-semibold uppercase text-[10px] ${levelColors[entry.level]}`}
      >
        {entry.level}
      </span>
      <span className="text-[#7b8794] shrink-0 w-[80px] truncate">
        {entry.source}
      </span>
      <span className="text-[#DCDCDC] break-all">{entry.message}</span>
    </div>
  );
}

export function LogViewer({ logs, autoScroll = true }: LogViewerProps) {
  const virtuosoRef = useRef<any>(null);
  const [filter, setFilter] = useState("");
  const [levelFilter, setLevelFilter] = useState<LogEntry["level"] | "all">("all");

  const filteredLogs = logs.filter((entry) => {
    if (levelFilter !== "all" && entry.level !== levelFilter) return false;
    if (filter && !entry.message.toLowerCase().includes(filter.toLowerCase()))
      return false;
    return true;
  });

  useEffect(() => {
    if (autoScroll && virtuosoRef.current) {
      virtuosoRef.current.scrollToIndex({
        index: filteredLogs.length - 1,
        behavior: "smooth",
      });
    }
  }, [filteredLogs.length, autoScroll]);

  return (
    <div className="h-full flex flex-col bg-[#0B0B0B] rounded-lg border border-[#171717] overflow-hidden">
      {/* Header */}
      <div className="h-8 flex items-center px-3 gap-2 bg-[#12121f] border-b border-[#171717] shrink-0">
        <span className="text-xs font-medium text-gray-300">Logs</span>
        <span className="text-[10px] text-gray-500">
          {filteredLogs.length}/{logs.length}
        </span>
        <div className="flex-1" />
        {/* Level filter */}
        <div className="flex gap-0.5">
          {(["all", "error", "warn", "info", "debug"] as const).map((level) => (
            <button
              key={level}
              onClick={() => setLevelFilter(level)}
              className={`px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors ${
                levelFilter === level
                  ? "bg-accent/20 text-accent"
                  : "text-gray-500 hover:text-gray-300"
              }`}
            >
              {level}
            </button>
          ))}
        </div>
        {/* Search */}
        <input
          type="text"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter..."
          className="w-32 px-2 py-0.5 bg-[#0B0B0B] border border-[#171717] rounded text-[10px] text-gray-300 placeholder-gray-600 focus:outline-none focus:border-accent/50"
        />
      </div>
      {/* Virtualized log list */}
      <div className="flex-1 min-h-0">
        <Virtuoso
          ref={virtuosoRef}
          data={filteredLogs}
          itemContent={(_index, entry) => <LogRow entry={entry} />}
          followOutput={autoScroll ? "smooth" : false}
          overscan={200}
        />
      </div>
    </div>
  );
}

/** Generate demo log entries for development/testing */
export function generateDemoLogs(count: number = 100): LogEntry[] {
  const levels: LogEntry["level"][] = ["info", "info", "info", "warn", "error", "debug"];
  const sources = ["server", "pty", "ws", "git", "auth", "orchestrator"];
  const messages = [
    "Session spawned successfully",
    "WebSocket connection established",
    "PTY output received (2048 bytes)",
    "Git worktree provisioned at .ia-hub/attempts/att-1",
    "Agent binary discovered: claude v2.1.251",
    "Scrollback replay sent (45KB)",
    "Health check passed",
    "Resize event: 120x40",
    "Credential store initialized",
    "Startup reconcile complete",
    "Rate limit warning: 80% quota used",
    "Connection timeout after 30s",
    "Process exited with code 0",
    "Task decomposed into 4 subtasks",
    "Provider account registered",
  ];

  const now = Date.now();
  return Array.from({ length: count }, (_, i) => ({
    id: `log-${i}`,
    timestamp: new Date(now - (count - i) * 1000).toISOString(),
    level: levels[i % levels.length],
    source: sources[i % sources.length],
    message: messages[i % messages.length],
  }));
}