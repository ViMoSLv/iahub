import { useState } from "react";

interface TaskItem {
  order: number;
  role: string;
  description: string;
  parallelizable: boolean;
  depends_on: number[];
  account_id: string | null;
  provider: string | null;
}

interface OrchestrateResult {
  objective: string;
  tasks: TaskItem[];
  assignments: TaskItem[];
  warning?: string;
  error?: string;
}

interface OrchestratorViewProps {
  port: number;
}

const ROLE_STYLES: Record<string, { border: string; icon: string; accent: string }> = {
  scout:    { border: "border-[#2472C8]/40", icon: "🔍", accent: "text-[#3B8EEA]" },
  coder:    { border: "border-[#0DBC79]/40", icon: "💻", accent: "text-[#23D18B]" },
  tester:   { border: "border-[#E5E510]/40", icon: "🧪", accent: "text-[#E5E510]" },
  reviewer: { border: "border-[#BC3FBC]/40", icon: "👁️", accent: "text-[#D670D6]" },
  shell:    { border: "border-[#555]/40",    icon: "⚡", accent: "text-[#A3A3A3]" },
};

export function OrchestratorView({ port }: OrchestratorViewProps) {
  const [objective, setObjective] = useState("");
  const [result, setResult] = useState<OrchestrateResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleOrchestrate = async () => {
    if (!objective.trim()) return;
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const resp = await fetch(`http://127.0.0.1:${port}/api/orchestrate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ objective: objective.trim() }),
      });
      if (resp.ok) {
        const data: OrchestrateResult = await resp.json();
        setResult(data);
      } else {
        setError(`HTTP ${resp.status}`);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="h-full flex flex-col bg-[#0B0B0B] overflow-hidden">
      {/* Header */}
      <div className="h-9 flex items-center px-4 gap-3 bg-[#121212] border-b border-[#171717] shrink-0">
        <span className="text-[12px] font-semibold text-[#DCDCDC]">🎯 Orchestrator</span>
        <span className="text-[10px] text-[#555]">Decompose objectives into agent pipelines</span>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Input */}
        <div className="flex gap-2">
          <input
            type="text"
            value={objective}
            onChange={(e) => setObjective(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleOrchestrate()}
            placeholder="Describe your objective... (e.g., Implement JWT authentication)"
            className="flex-1 px-3 py-2 bg-[#121212] border border-[#232323] rounded-lg text-[13px] text-[#DCDCDC] placeholder-[#555] focus:outline-none focus:border-[#007acc] transition-colors"
          />
          <button
            onClick={handleOrchestrate}
            disabled={loading || !objective.trim()}
            className="px-4 py-2 bg-[#007acc]/15 text-[#007acc] hover:bg-[#007acc]/25 disabled:opacity-40 rounded-lg text-[12px] font-medium transition-colors border border-[#007acc]/20"
          >
            {loading ? "..." : "Decompose"}
          </button>
        </div>

        {/* Error */}
        {error && (
          <div className="px-3 py-2 bg-[#f44747]/10 border border-[#f44747]/30 rounded-lg text-xs text-[#f44747]">
            {error}
          </div>
        )}

        {/* Pipeline visualization */}
        {result && (
          <div className="space-y-3">
            {/* Warning */}
            {result.warning && (
              <div className="px-3 py-2 bg-[#F0C24B]/10 border border-[#F0C24B]/30 rounded-lg text-xs text-[#F0C24B]">
                ⚠️ {result.warning}
              </div>
            )}

            {/* Pipeline flow */}
            <div className="flex items-start gap-0 overflow-x-auto pb-2">
              {result.tasks.map((task, i) => {
                const style = ROLE_STYLES[task.role] || ROLE_STYLES.shell;
                return (
                  <div key={i} className="flex items-start shrink-0">
                    {/* Arrow connector */}
                    {i > 0 && (
                      <div className="flex items-center pt-8 px-1">
                        <div className="w-5 h-px bg-[#333]" />
                        <svg width="6" height="8" viewBox="0 0 6 8" fill="none" className="-ml-px">
                          <path d="M0 0l6 4-6 4V0z" fill="#333" />
                        </svg>
                      </div>
                    )}

                    {/* Task card — dark surface, subtle colored left border only */}
                    <div
                      className={`w-44 rounded-lg border border-[#232323] bg-[#121212] overflow-hidden ${style.border}`}
                      style={{ borderLeftWidth: "2px" }}
                    >
                      <div className="p-3">
                        <div className="flex items-center gap-1.5 mb-2">
                          <span className="text-xs">{style.icon}</span>
                          <span className={`text-[10px] font-bold uppercase tracking-wider ${style.accent}`}>
                            {task.role}
                          </span>
                          <span className="ml-auto text-[9px] text-[#555]">#{task.order}</span>
                        </div>
                        <p className="text-[11px] leading-relaxed text-[#A3A3A3] line-clamp-3">
                          {task.description}
                        </p>
                        {task.account_id && (
                          <div className="mt-2 pt-2 border-t border-[#232323]">
                            <div className="text-[10px] text-[#7A7A7A] truncate flex items-center gap-1">
                              <span className="w-1 h-1 rounded-full bg-[#4ADE80] shrink-0" />
                              {task.account_id.substring(0, 8)}...
                            </div>
                            <div className="text-[9px] text-[#555] mt-0.5">{task.provider}</div>
                          </div>
                        )}
                        {task.depends_on.length > 0 && (
                          <div className="mt-1.5 text-[9px] text-[#555]">
                            depends on #{task.depends_on.join(", #")}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Summary */}
            <div className="px-3 py-2.5 bg-[#121212] rounded-lg border border-[#232323]">
              <div className="text-[10px] text-[#555] uppercase tracking-wider mb-1 font-semibold">Summary</div>
              <div className="text-xs text-[#A3A3A3]">
                {result.tasks.length} tasks • {result.assignments.length} assigned •{" "}
                {result.tasks.filter((t) => t.account_id).length}/{result.tasks.length} accounts bound
              </div>
            </div>
          </div>
        )}

        {/* Empty state */}
        {!result && !loading && !error && (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-14 h-14 rounded-2xl bg-[#121212] border border-[#232323] flex items-center justify-center mb-4">
              <span className="text-2xl">🎯</span>
            </div>
            <div className="text-sm text-[#A3A3A3] mb-1">Enter an objective above</div>
            <div className="text-xs text-[#555] max-w-xs">
              The orchestrator will decompose it into a scout → coder → tester → reviewer pipeline
            </div>
          </div>
        )}
      </div>
    </div>
  );
}