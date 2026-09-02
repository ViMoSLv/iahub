import { useState, useEffect, useCallback } from "react";
import { Sidebar } from "./components/Sidebar";
import { PanelGrid } from "./components/PanelGrid";
import { Header } from "./components/Header";
import { Onboarding } from "./pages/Onboarding";
import { useBackend } from "./hooks/useBackend";
import type { HealthResponse, SessionInfo, ProjectInfo, ProviderAccountInfo } from "./lib/types";

type LayoutMode = "grid" | "spotlight" | "sidebar";
type AppPhase = "loading" | "onboarding" | "ready";

export default function App() {
  const [phase, setPhase] = useState<AppPhase>("loading");
  const [layout, setLayout] = useState<LayoutMode>("grid");
  const [sessions] = useState<SessionInfo[]>([]);
  const [projects] = useState<ProjectInfo[]>([]);
  const [accounts] = useState<ProviderAccountInfo[]>([]);
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const { connected, port } = useBackend();

  // Poll health until backend is ready
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const resp = await fetch(`http://127.0.0.1:${port || 8080}/health`);
        if (resp.ok) {
          const data: HealthResponse = await resp.json();
          if (data.status === "ready") {
            setPhase(data.onboarding_required ? "onboarding" : "ready");
            clearInterval(interval);
          }
        }
      } catch {
        // Backend not ready yet
      }
    }, 2000);
    return () => clearInterval(interval);
  }, [port]);

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
      />
      <div className="flex-1 flex overflow-hidden">
        <Sidebar
          projects={projects}
          accounts={accounts}
          activeProjectId={activeProjectId}
          onProjectSelect={setActiveProjectId}
        />
        <main className="flex-1 overflow-hidden p-[var(--panel-gap)]">
          <PanelGrid
            sessions={sessions}
            layout={layout}
            port={port || 8080}
          />
        </main>
      </div>
    </div>
  );
}