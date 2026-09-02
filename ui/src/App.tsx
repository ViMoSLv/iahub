import { useState, useEffect, useCallback } from "react";
import { Sidebar } from "./components/Sidebar";
import { PanelGrid } from "./components/PanelGrid";
import { Header } from "./components/Header";
import { Onboarding } from "./pages/Onboarding";
import { OrchestratorView } from "./components/OrchestratorView";
import { useBackend } from "./hooks/useBackend";
import type { SessionInfo, ProjectInfo, ProviderAccountInfo } from "./lib/types";

type LayoutMode = "grid" | "spotlight" | "sidebar";
type AppPhase = "loading" | "onboarding" | "ready";
type ActiveView = "terminals" | "orchestrator";

export default function App() {
  const [phase, setPhase] = useState<AppPhase>("loading");
  const [layout, setLayout] = useState<LayoutMode>("grid");
  const [activeView, setActiveView] = useState<ActiveView>("terminals");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [projects] = useState<ProjectInfo[]>([]);
  const [accounts, setAccounts] = useState<ProviderAccountInfo[]>([]);
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const [agents, setAgents] = useState<Array<{ name: string; binary: string; status: string }>>([]);
  const { connected, port, health } = useBackend();

  const backendPort = port || 8080;
  const baseUrl = `http://127.0.0.1:${backendPort}`;

  // Transition from loading → ready when useBackend confirms connection
  useEffect(() => {
    if (connected && phase === "loading") {
      setPhase(health?.onboarding_required ? "onboarding" : "ready");
    }
  }, [connected, health, phase]);

  // Absolute fallback: force ready after 5s regardless of connection state.
  // This prevents the "Connecting to backend..." screen from hanging forever
  // when the backend is slow to start or the first fetch fails.
  useEffect(() => {
    if (phase !== "loading") return;
    const timer = setTimeout(() => {
      setPhase("ready");
    }, 5000);
    return () => clearTimeout(timer);
  }, [phase]);

  // Fetch agents on mount
  useEffect(() => {
    if (phase !== "ready") return;
    fetch(`${baseUrl}/api/agents`)
      .then((r) => r.json())
      .then((data) => setAgents(data))
      .catch(() => {});
  }, [phase, baseUrl]);

  // Poll sessions every 3s
  useEffect(() => {
    if (phase !== "ready") return;
    const poll = async () => {
      try {
        const resp = await fetch(`${baseUrl}/api/sessions`);
        if (resp.ok) {
          const data = await resp.json();
          setSessions(data);
        }
      } catch {
        // ignore
      }
    };
    poll();
    const interval = setInterval(poll, 3000);
    return () => clearInterval(interval);
  }, [phase, baseUrl]);

  // Fetch accounts on mount
  useEffect(() => {
    if (phase !== "ready") return;
    fetch(`${baseUrl}/api/accounts`)
      .then((r) => r.json())
      .then((data) => setAccounts(data))
      .catch(() => {});
  }, [phase, baseUrl]);

  const handleSpawnSession = useCallback(
    async (agentBinary: string, accountId?: string) => {
      try {
        const body: Record<string, string> = { agent_binary: agentBinary };
        if (accountId) body.account_id = accountId;
        const resp = await fetch(`${baseUrl}/api/sessions`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        });
        if (resp.ok) {
          const data = await resp.json();
          setSessions((prev) => [
            ...prev,
            {
              id: data.session_id,
              account_id: accountId || "auto",
              provider: agentBinary,
              agent_binary: agentBinary,
              status: "active",
              workspace_path: data.workspace_path,
            },
          ]);
        }
      } catch {
        // ignore
      }
    },
    [baseUrl],
  );

  const handleAddAccount = useCallback(
    async (provider: string, label: string) => {
      try {
        const resp = await fetch(`${baseUrl}/api/accounts`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ provider, label }),
        });
        if (resp.ok) {
          const data = await resp.json();
          setAccounts((prev) => [
            ...prev,
            {
              id: data.id,
              provider: data.provider,
              label: data.label,
              status: data.status,
              max_concurrent_sessions: data.max_concurrent_sessions,
              active_sessions: data.active_sessions,
            },
          ]);
        }
      } catch {
        // ignore
      }
    },
    [baseUrl],
  );

  const handleOnboardingComplete = useCallback(() => {
    setPhase("ready");
  }, []);

  if (phase === "loading") {
    return (
      <div className="h-full w-full flex items-center justify-center bg-surface">
        <div className="text-center">
          <div className="text-2xl font-bold text-accent mb-2">IA-Hub</div>
          <div className="text-status-idle text-sm">Connecting to backend...</div>
          <div className="mt-4 w-8 h-8 border-2 border-accent border-t-transparent rounded-full animate-spin mx-auto" />
        </div>
      </div>
    );
  }

  if (phase === "onboarding") {
    return <Onboarding onComplete={handleOnboardingComplete} port={port || 8080} />;
  }

  return (
    <div className="h-full w-full flex flex-col bg-surface overflow-hidden">
      <Header
        layout={layout}
        onLayoutChange={setLayout}
        sessionCount={sessions.length}
        connected={connected}
        agents={agents}
        accounts={accounts}
        onSpawnSession={handleSpawnSession}
      />
      <div className="flex-1 flex overflow-hidden">
        <Sidebar
          projects={projects}
          accounts={accounts}
          activeProjectId={activeProjectId}
          onProjectSelect={setActiveProjectId}
          onAddAccount={handleAddAccount}
        />
        <main className="flex-1 overflow-hidden flex flex-col">
          {/* View switcher tabs */}
          <div className="h-8 flex items-center px-2 gap-1 bg-surface-raised border-b border-[var(--border-color)] shrink-0">
            <button
              onClick={() => setActiveView("terminals")}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                activeView === "terminals"
                  ? "bg-accent/20 text-accent"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              Terminals
            </button>
            <button
              onClick={() => setActiveView("orchestrator")}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                activeView === "orchestrator"
                  ? "bg-accent/20 text-accent"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              Orchestrator
            </button>
          </div>
          {/* Active view content */}
          <div className="flex-1 overflow-hidden p-[var(--panel-gap)]">
            {activeView === "terminals" ? (
              <PanelGrid
                sessions={sessions}
                layout={layout}
                port={port || 8080}
                agents={agents}
                onSpawnSession={handleSpawnSession}
              />
            ) : (
              <OrchestratorView port={port || 8080} />
            )}
          </div>
        </main>
      </div>
    </div>
  );
}