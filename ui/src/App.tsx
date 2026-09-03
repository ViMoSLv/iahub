import { useState, useEffect, useCallback, useRef } from "react";
import { Sidebar } from "./components/Sidebar";
import { PanelGrid } from "./components/PanelGrid";
import { Header } from "./components/Header";
import { Onboarding } from "./pages/Onboarding";
import { OrchestratorView } from "./components/OrchestratorView";
import { FileExplorer, buildFileTree } from "./components/FileExplorer";
import { ActivityBar } from "./components/ActivityBar";
import { useBackend } from "./hooks/useBackend";
import type { SessionInfo, ProjectInfo, ProviderAccountInfo } from "./lib/types";

type LayoutMode = "grid" | "spotlight" | "sidebar";
type AppPhase = "loading" | "onboarding" | "ready";
type ActiveView = "terminals" | "orchestrator";
type SidePanel = "explorer" | "sessions" | "orchestrator" | null;

interface FileNode {
  name: string;
  path: string;
  isDirectory: boolean;
  children?: FileNode[];
  expanded?: boolean;
}

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

  // File explorer state
  const [sidePanel, setSidePanel] = useState<SidePanel>("sessions");
  const [fileTree, setFileTree] = useState<FileNode | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const dragCounterRef = useRef(0);

  const backendPort = port || 8080;
  const baseUrl = `http://127.0.0.1:${backendPort}`;

  // Transition from loading → ready when useBackend confirms connection
  useEffect(() => {
    if (connected && phase === "loading") {
      setPhase(health?.onboarding_required ? "onboarding" : "ready");
    }
  }, [connected, health, phase]);

  // Absolute fallback: force ready after 2s regardless of connection state.
  useEffect(() => {
    if (phase !== "loading") return;
    const timer = setTimeout(() => {
      setPhase("ready");
    }, 2000);
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

  // Drag & Drop handlers for folder drop
  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current++;
    if (e.dataTransfer.types.includes("Files")) {
      setIsDragOver(true);
    }
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current--;
    if (dragCounterRef.current === 0) {
      setIsDragOver(false);
    }
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleDrop = useCallback(async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current = 0;
    setIsDragOver(false);

    const items = e.dataTransfer.items;
    if (!items || items.length === 0) return;

    // Use webkitGetAsEntry to traverse directories
    const entries: FileSystemEntry[] = [];
    for (let i = 0; i < items.length; i++) {
      const entry = items[i].webkitGetAsEntry?.();
      if (entry) entries.push(entry);
    }

    if (entries.length === 0) return;

    // Traverse the first directory entry
    const allPaths: string[] = [];
    let rootName = "dropped-folder";

    const traverseEntry = async (entry: FileSystemEntry, basePath: string): Promise<void> => {
      if (entry.isFile) {
        allPaths.push(basePath + entry.name);
      } else if (entry.isDirectory) {
        if (!basePath) rootName = entry.name;
        const dirReader = (entry as FileSystemDirectoryEntry).createReader();
        const readEntries = (): Promise<FileSystemEntry[]> =>
          new Promise((resolve) => dirReader.readEntries(resolve));

        let batch = await readEntries();
        while (batch.length > 0) {
          for (const child of batch) {
            await traverseEntry(child, basePath + entry.name + "/");
          }
          batch = await readEntries();
        }
      }
    };

    for (const entry of entries) {
      await traverseEntry(entry, "");
    }

    if (allPaths.length > 0 || entries.some(e => e.isDirectory)) {
      const tree = buildFileTree(allPaths, rootName);
      setFileTree(tree);
      setSidePanel("explorer");
    }
  }, []);

  const handlePanelToggle = useCallback((panel: "explorer" | "sessions" | "orchestrator") => {
    setSidePanel((prev) => (prev === panel ? null : panel));
    if (panel === "orchestrator") {
      setActiveView("orchestrator");
    } else {
      setActiveView("terminals");
    }
  }, []);

  if (phase === "loading") {
    return (
      <div className="h-full w-full flex items-center justify-center bg-[#0B0B0B]">
        <div className="text-center">
          <div className="text-2xl font-bold text-[#007acc] mb-2 tracking-tight">IA-Hub</div>
          <div className="text-[#7A7A7A] text-sm">Connecting to backend...</div>
          <div className="mt-4 w-7 h-7 border-2 border-[#007acc] border-t-transparent rounded-full animate-spin mx-auto" />
          <button
            onClick={() => setPhase("ready")}
            className="mt-6 px-4 py-1.5 text-xs text-[#7A7A7A] hover:text-[#C9C9C9] border border-[#232323] rounded-lg hover:border-[#555] transition-colors"
          >
            Skip → Enter Panel
          </button>
        </div>
      </div>
    );
  }

  if (phase === "onboarding") {
    return <Onboarding onComplete={handleOnboardingComplete} port={port || 8080} />;
  }

  return (
    <div
      className="h-full w-full flex flex-col bg-[#0B0B0B] overflow-hidden relative"
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {/* Drop overlay */}
      {isDragOver && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-[#0B0B0B]/90 backdrop-blur-sm pointer-events-none">
          <div className="drop-overlay border-2 border-dashed border-[#007acc] rounded-xl p-12 text-center">
            <div className="text-4xl mb-3">📂</div>
            <div className="text-[#DCDCDC] text-lg font-medium">Solte a pasta aqui</div>
            <div className="text-[#7A7A7A] text-sm mt-1">para explorar os arquivos</div>
          </div>
        </div>
      )}

      {/* Header */}
      <Header
        layout={layout}
        onLayoutChange={setLayout}
        sessionCount={sessions.length}
        connected={connected}
        agents={agents}
        accounts={accounts}
        onSpawnSession={handleSpawnSession}
      />

      {/* Main body */}
      <div className="flex-1 flex overflow-hidden">
        {/* Activity Bar */}
        <ActivityBar
          activePanel={sidePanel}
          onPanelToggle={handlePanelToggle}
          hasFolder={fileTree !== null}
        />

        {/* Side Panel */}
        {sidePanel && (
          <div className="w-[var(--sidebar-width)] border-r border-[#171717] flex flex-col shrink-0 overflow-hidden sidebar-transition">
            {sidePanel === "explorer" ? (
              <FileExplorer
                rootFolder={fileTree}
                onClose={() => setSidePanel(null)}
              />
            ) : sidePanel === "sessions" ? (
              <Sidebar
                projects={projects}
                accounts={accounts}
                activeProjectId={activeProjectId}
                onProjectSelect={setActiveProjectId}
                onAddAccount={handleAddAccount}
              />
            ) : null}
          </div>
        )}

        {/* Main content */}
        <main className="flex-1 overflow-hidden flex flex-col min-w-0">
          {/* View switcher tabs */}
          <div className="h-8 flex items-center px-2 gap-0.5 bg-[#121212] border-b border-[#171717] shrink-0">
            <button
              onClick={() => { setActiveView("terminals"); }}
              className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                activeView === "terminals"
                  ? "bg-[#1A1A1A] text-[#DCDCDC] border border-[#232323]"
                  : "text-[#7A7A7A] hover:text-[#C9C9C9] hover:bg-[#161616]"
              }`}
            >
              Terminals
              {sessions.length > 0 && (
                <span className="ml-1.5 text-[10px] opacity-60">{sessions.length}</span>
              )}
            </button>
            <button
              onClick={() => { setActiveView("orchestrator"); }}
              className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                activeView === "orchestrator"
                  ? "bg-[#1A1A1A] text-[#DCDCDC] border border-[#232323]"
                  : "text-[#7A7A7A] hover:text-[#C9C9C9] hover:bg-[#161616]"
              }`}
            >
              Orchestrator
            </button>
            <div className="flex-1" />
            <span className="text-[10px] text-[#555] mr-2">
              {fileTree ? `📁 ${fileTree.name}` : "Arraste uma pasta para explorar"}
            </span>
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