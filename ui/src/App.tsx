import { useState, useEffect, useCallback } from "react";
import { Sidebar } from "./components/Sidebar";
import { PanelGrid } from "./components/PanelGrid";
import { Header } from "./components/Header";
import { Onboarding } from "./pages/Onboarding";
import { useBackend } from "./hooks/useBackend";
import type { SessionInfo, ProjectInfo, ProviderAccountInfo } from "./lib/types";

type LayoutMode = "grid" | "spotlight" | "sidebar";
type AppPhase = "loading" | "onboarding" | "ready";

export default function App() {
  const [phase, setPhase] = useState<AppPhase>("loading");
  const [layout, setLayout] = useState<LayoutMode>("grid");
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

  // Fallback: if connected but health hasn't arrived yet after 3s, go ready anyway
  useEffect(() => {
    if (connected && phase === "loading") {
      const timer = setTimeout(() => {
        setPhase("ready");
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [connected, phase]);

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
    async (agentBinary: string) => {
      try {
        const resp = await fetch(`${baseUrl}/api/sessions`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ agent_binary: agentBinary }),
        });
        if (resp.ok) {
          const data = await resp.json();
          setSessions((prev) => [
            ...prev,
            {
              id: data.session_id,
              account_id: "default",
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
        <main className="flex-1 overflow-hidden p-[var(--panel-gap)]">
          <PanelGrid
            sessions={sessions}
            layout={layout}
            port={port || 8080}
            agents={agents}
            onSpawnSession={handleSpawnSession}
          />
        </main>
      </div>
    </div>
  );
}