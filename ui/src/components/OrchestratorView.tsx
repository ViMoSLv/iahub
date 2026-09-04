import { useState, useCallback } from "react";

interface OrchestratorViewProps {
  port: number;
}

interface TaskAssignment {
  order: number;
  role: string;
  provider: string;
  account_id: string;
  description: string;
  depends_on: number[];
  parallelizable: boolean;
}

interface OrchestrateResult {
  objective: string;
  tasks: TaskAssignment[];
  assignments: TaskAssignment[];
}

type TaskStatus = "pending" | "running" | "done" | "failed";

const roleIcons: Record<string, string> = {
  scout: "🔍",
  coder: "💻",
  tester: "🧪",
  reviewer: "👁️",
};

const roleColors: Record<string, string> = {
  scout: "border-[#48cae4] bg-[#48cae410]",
  coder: "border-[#a78bfa] bg-[#a78bfa10]",
  tester: "border-[#34d399] bg-[#34d39910]",
  reviewer: "border-[#f0c040] bg-[#f0c04010]",
};

const statusBadge: Record<TaskStatus, { label: string; color: string }> = {
  pending: { label: "⏳ Pending", color: "text-gray-500" },
  running: { label: "🔄 Running", color: "text-[#48cae4]" },
  done: { label: "✅ Done", color: "text-[#34d399]" },
  failed: { label: "❌ Failed", color: "text-[#e94560]" },
};

export function OrchestratorView({ port }: OrchestratorViewProps) {
  const [objective, setObjective] = useState("");
  const [result, setResult] = useState<OrchestrateResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [taskStatuses, setTaskStatuses] = useState<Record<number, TaskStatus>>({});

  const handleOrchestrate = useCallback(async () => {
    if (!objective.trim()) return;
    setLoading(true);
    setError("");
    try {
      const res = await fetch(`http://127.0.0.1:${port}/api/orchestrate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ objective: objective.trim() }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data: OrchestrateResult = await res.json();
      setResult(data);
      // Initialize all tasks as pending
      const statuses: Record<number, TaskStatus> = {};
      data.tasks.forEach((t) => { statuses[t.order] = "pending"; });
      setTaskStatuses(statuses);
    } catch (e: any) {
      setError(e.message || "Failed to orchestrate");
    } finally {
      setLoading(false);
    }
  }, [objective, port]);

  const simulateProgress = useCallback(() => {
    if (!result) return;
    const tasks = [...result.tasks].sort((a, b) => a.order - b.order);
    let idx = 0;
    const advance = () => {
      if (idx >= tasks.length) return;
      const task = tasks[idx];
      setTaskStatuses((prev) => ({ ...prev, [task.order]: "running" }));
      setTimeout(() => {
        setTaskStatuses((prev) => ({ ...prev, [task.order]: "done" }));
        idx++;
        if (idx < tasks.length) {
          setTimeout(advance, 300);
        }
      }, 800 + Math.random() * 600);
    };
    advance();
  }, [result]);

  return (
    <div className="h-full flex flex-col gap-4 p-4 overflow-auto">
      {/* Input */}
      <div className="flex gap-2 items-start">
        <div className="flex-1">
          <label className="text-xs text-gray-500 mb-1 block">Objective</label>
          <textarea
            value={objective}
            onChange={(e) => setObjective(e.target.value)}
            placeholder="e.g., Implement JWT authentication with refresh tokens"
            className="w-full px-3 py-2 bg-[#12121f] border border-[#232323] rounded-lg text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-[#48cae4]/50 resize-none h-20"
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) handleOrchestrate();
            }}
          />
        </div>
        <button
          onClick={handleOrchestrate}
          disabled={loading || !objective.trim()}
          className="mt-5 px-4 py-2 bg-[#48cae4]/20 text-[#48cae4] border border-[#48cae4]/30 rounded-lg text-sm font-medium hover:bg-[#48cae4]/30 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
        >
          {loading ? "Decomposing..." : "Orchestrate"}
        </button>
      </div>

      {error && (
        <div className="px-3 py-2 bg-[#e9456010] border border-[#e94560]/30 rounded-lg text-sm text-[#e94560]">
          {error}
        </div>
      )}

      {/* Pipeline visualization */}
      {result && (
        <div className="flex-1">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-medium text-gray-300">
              Pipeline: <span className="text-gray-500">{result.objective}</span>
            </h3>
            <button
              onClick={simulateProgress}
              className="px-3 py-1 text-xs bg-[#34d399]/20 text-[#34d399] border border-[#34d399]/30 rounded hover:bg-[#34d399]/30 transition-colors"
            >
              ▶ Simulate
            </button>
          </div>

          {/* Flow chart */}
          <div className="flex items-center gap-0 overflow-x-auto pb-4">
            {result.tasks.map((task, i) => {
              const status = taskStatuses[task.order] || "pending";
              const badge = statusBadge[status];
              return (
                <div key={task.order} className="flex items-center">
                  {/* Arrow connector */}
                  {i > 0 && (
                    <div className="flex items-center px-1">
                      <div className={`w-8 h-0.5 ${
                        taskStatuses[result.tasks[i - 1].order] === "done"
                          ? "bg-[#34d399]"
                          : "bg-[#232323]"
                      }`} />
                      <div className={`w-0 h-0 border-t-4 border-b-4 border-l-6 border-t-transparent border-b-transparent ${
                        taskStatuses[result.tasks[i - 1].order] === "done"
                          ? "border-l-[#34d399]"
                          : "border-l-[#232323]"
                      }`} />
                    </div>
                  )}
                  {/* Task card */}
                  <div
                    className={`min-w-[180px] max-w-[220px] rounded-lg border p-3 transition-all ${
                      roleColors[task.role] || "border-[#232323] bg-[#12121f]"
                    } ${status === "running" ? "ring-1 ring-[#48cae4]/50 scale-[1.02]" : ""}`}
                  >
                    <div className="flex items-center gap-2 mb-1.5">
                      <span className="text-lg">{roleIcons[task.role] || ""}</span>
                      <span className="text-xs font-semibold text-gray-200 uppercase">
                        {task.role}
                      </span>
                      <span className={`ml-auto text-[10px] font-medium ${badge.color}`}>
                        {badge.label}
                      </span>
                    </div>
                    <p className="text-[11px] text-gray-400 leading-relaxed line-clamp-3">
                      {task.description}
                    </p>
                    <div className="mt-2 flex items-center gap-1.5">
                      <span className="text-[10px] text-gray-600">
                        {task.provider}
                      </span>
                      <span className="text-[10px] text-gray-700">•</span>
                      <span className="text-[10px] text-gray-600 truncate max-w-[80px]" title={task.account_id}>
                        {task.account_id.slice(0, 8)}
                      </span>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>

          {/* Task details table */}
          <div className="mt-4 border border-[#171717] rounded-lg overflow-hidden">
            <table className="w-full text-xs">
              <thead>
                <tr className="bg-[#12121f] text-gray-500">
                  <th className="text-left px-3 py-2 font-medium">#</th>
                  <th className="text-left px-3 py-2 font-medium">Role</th>
                  <th className="text-left px-3 py-2 font-medium">Provider</th>
                  <th className="text-left px-3 py-2 font-medium">Account</th>
                  <th className="text-left px-3 py-2 font-medium">Description</th>
                  <th className="text-left px-3 py-2 font-medium">Deps</th>
                  <th className="text-left px-3 py-2 font-medium">Status</th>
                </tr>
              </thead>
              <tbody>
                {result.tasks.map((task) => {
                  const status = taskStatuses[task.order] || "pending";
                  const badge = statusBadge[status];
                  return (
                    <tr key={task.order} className="border-t border-[#171717] hover:bg-white/[0.02]">
                      <td className="px-3 py-2 text-gray-500">{task.order}</td>
                      <td className="px-3 py-2">
                        <span className="flex items-center gap-1">
                          {roleIcons[task.role]} {task.role}
                        </span>
                      </td>
                      <td className="px-3 py-2 text-gray-400">{task.provider}</td>
                      <td className="px-3 py-2 text-gray-500 font-mono text-[10px]">
                        {task.account_id.slice(0, 8)}
                      </td>
                      <td className="px-3 py-2 text-gray-300 max-w-[300px] truncate">
                        {task.description}
                      </td>
                      <td className="px-3 py-2 text-gray-600">
                        {task.depends_on.length > 0 ? task.depends_on.join(", ") : "—"}
                      </td>
                      <td className={`px-3 py-2 font-medium ${badge.color}`}>
                        {badge.label}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* INV-006 verification badge */}
          {result.tasks.length >= 2 && (() => {
            const coder = result.tasks.find((t) => t.role === "coder");
            const reviewer = result.tasks.find((t) => t.role === "reviewer");
            if (coder && reviewer) {
              const compliant = coder.account_id !== reviewer.account_id;
              return (
                <div className={`mt-3 px-3 py-2 rounded-lg border text-xs flex items-center gap-2 ${
                  compliant
                    ? "bg-[#34d39910] border-[#34d399]/30 text-[#34d399]"
                    : "bg-[#e9456010] border-[#e94560]/30 text-[#e94560]"
                }`}>
                  <span>{compliant ? "✅" : "❌"}</span>
                  <span>
                    INV-006 Self-Review Prevention: coder ({coder.account_id.slice(0, 8)})
                    {compliant ? " ≠ " : " = "}
                    reviewer ({reviewer.account_id.slice(0, 8)})
                    {compliant ? " — COMPLIANT" : " — VIOLATION"}
                  </span>
                </div>
              );
            }
            return null;
          })()}
        </div>
      )}

      {/* Empty state */}
      {!result && !loading && (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center text-gray-600">
            <div className="text-4xl mb-3">🎯</div>
            <p className="text-sm">Enter an objective and click Orchestrate</p>
            <p className="text-xs mt-1 text-gray-700">
              The pipeline will decompose into scout → coder → tester → reviewer
            </p>
          </div>
        </div>
      )}
    </div>
  );
}