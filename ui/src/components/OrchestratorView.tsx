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

const ROLE_COLORS: Record<string, string> = {
  scout: "bg-blue-500/20 text-blue-400 border-blue-500/30",
  coder: "bg-green-500/20 text-green-400 border-green-500/30",
  tester: "bg-yellow-500/20 text-yellow-400 border-yellow-500/30",
  reviewer: "bg-purple-500/20 text-purple-400 border-purple-500/30",
  shell: "bg-gray-500/20 text-gray-400 border-gray-500/30",
};

const ROLE_ICONS: Record<string, string> = {
  scout: "🔍",
  coder: "💻",
  tester: "🧪",
  reviewer: "👁️",
  shell: "⚡",
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
    <div className="h-full flex flex-col bg-surface overflow-hidden">
      {/* Header */}
      <div className="h-10 flex items-center px-4 gap-3 bg-surface-raised border-b border-[var(--border-color)] shrink-0">
        <span className="text-sm font-semibold text-gray-200"> Orchestrator</span>
        <span className="text-[10px] text-gray-500">Decompose objectives into agent pipelines</span>
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
            className="flex-1 px-3 py-2 bg-surface-raised border border-[var(--border-color)] rounded-lg text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-accent"
          />
          <button
            onClick={handleOrchestrate}
            disabled={loading || !objective.trim()}
            className="px-4 py-2 bg-accent hover:bg-accent/90 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
          >
            {loading ? "..." : "Decompose"}
          </button>
        </div>

        {/* Error */}
        {error && (
          <div className="px-3 py-2 bg-red-500/10 border border-red-500/30 rounded-lg text-xs text-red-400">
            {error}
          </div>
        )}

        {/* Pipeline visualization */}
        {result && (
          <div className="space-y-3">
            {/* Warning */}
            {result.warning && (
              <div className="px-3 py-2 bg-yellow-500/10 border border-yellow-500/30 rounded-lg text-xs text-yellow-400">
                ⚠️ {result.warning}
              </div>
            )}

            {/* Pipeline flow */}
            <div className="flex items-start gap-2 overflow-x-auto pb-2">
              {result.tasks.map((task, i) => (
                <div key={i} className="flex items-start gap-2 shrink-0">
                  {/* Arrow connector */}
                  {i > 0 && (
                    <div className="flex items-center h-full pt-6">
                      <div className="w-6 h-0.5 bg-gray-600" />
                      <div className="w-0 h-0 border-t-4 border-b-4 border-l-6 border-transparent border-l-gray-600" />
                    </div>
                  )}

                  {/* Task card */}
                  <div
                    className={`w-48 rounded-lg border p-3 ${ROLE_COLORS[task.role] || "bg-gray-500/20 text-gray-400 border-gray-500/30"}`}
                  >
                    <div className="flex items-center gap-1.5 mb-1.5">
                      <span className="text-sm">{ROLE_ICONS[task.role] || "❓"}</span>
                      <span className="text-xs font-semibold uppercase tracking-wider">
                        {task.role}
                      </span>
                      <span className="ml-auto text-[10px] opacity-50">#{task.order}</span>
                    </div>
                    <p className="text-[11px] leading-relaxed opacity-80 line-clamp-3">
                      {task.description}
                    </p>
                    {task.account_id && (
                      <div className="mt-2 pt-1.5 border-t border-current/20">
                        <div className="text-[10px] opacity-60 truncate">
                          📋 {task.account_id.substring(0, 8)}...
                        </div>
                        <div className="text-[10px] opacity-40">{task.provider}</div>
                      </div>
                    )}
                    {task.depends_on.length > 0 && (
                      <div className="mt-1 text-[9px] opacity-40">
                        depends on #{task.depends_on.join(", #")}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {/* Summary */}
            <div className="px-3 py-2 bg-surface-raised rounded-lg border border-[var(--border-color)]">
              <div className="text-[10px] text-gray-500 uppercase tracking-wider mb-1">Summary</div>
              <div className="text-xs text-gray-300">
                {result.tasks.length} tasks • {result.assignments.length} assigned •{" "}
                {result.tasks.filter((t) => t.account_id).length}/{result.tasks.length} accounts bound
              </div>
            </div>
          </div>
        )}

        {/* Empty state */}
        {!result && !loading && !error && (
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <div className="text-3xl mb-3">🎯</div>
            <div className="text-sm text-gray-400 mb-1">Enter an objective above</div>
            <div className="text-xs text-gray-600">
              The orchestrator will decompose it into a scout → coder → tester → reviewer pipeline
            </div>
          </div>
        )}
      </div>
    </div>
  );
}