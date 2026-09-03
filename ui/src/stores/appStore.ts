import { create } from "zustand";
import type { SessionInfo, ProjectInfo, ProviderAccountInfo } from "../lib/types";

type LayoutMode = "grid" | "spotlight" | "sidebar";
type AppPhase = "loading" | "onboarding" | "ready";
type ActiveView = "terminals" | "orchestrator";
type SidePanel = "explorer" | "sessions" | "orchestrator" | null;

interface AgentInfo {
  name: string;
  binary: string;
  status: string;
}

interface AppState {
  // Core state
  phase: AppPhase;
  layout: LayoutMode;
  activeView: ActiveView;
  sidePanel: SidePanel;
  connected: boolean;
  port: number;

  // Data
  sessions: SessionInfo[];
  projects: ProjectInfo[];
  accounts: ProviderAccountInfo[];
  agents: AgentInfo[];
  activeProjectId: string | null;

  // UI state
  commandPaletteOpen: boolean;
  isDragOver: boolean;

  // Actions
  setPhase: (phase: AppPhase) => void;
  setLayout: (layout: LayoutMode) => void;
  setActiveView: (view: ActiveView) => void;
  setSidePanel: (panel: SidePanel) => void;
  setConnected: (connected: boolean) => void;
  setPort: (port: number) => void;
  setSessions: (sessions: SessionInfo[]) => void;
  setProjects: (projects: ProjectInfo[]) => void;
  setAccounts: (accounts: ProviderAccountInfo[]) => void;
  setAgents: (agents: AgentInfo[]) => void;
  setActiveProjectId: (id: string | null) => void;
  toggleCommandPalette: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setIsDragOver: (dragOver: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  // Initial state
  phase: "loading",
  layout: "grid",
  activeView: "terminals",
  sidePanel: "sessions",
  connected: false,
  port: 8080,
  sessions: [],
  projects: [],
  accounts: [],
  agents: [],
  activeProjectId: null,
  commandPaletteOpen: false,
  isDragOver: false,

  // Actions
  setPhase: (phase) => set({ phase }),
  setLayout: (layout) => set({ layout }),
  setActiveView: (activeView) => set({ activeView }),
  setSidePanel: (sidePanel) => set({ sidePanel }),
  setConnected: (connected) => set({ connected }),
  setPort: (port) => set({ port }),
  setSessions: (sessions) => set({ sessions }),
  setProjects: (projects) => set({ projects }),
  setAccounts: (accounts) => set({ accounts }),
  setAgents: (agents) => set({ agents }),
  setActiveProjectId: (activeProjectId) => set({ activeProjectId }),
  toggleCommandPalette: () =>
    set((state) => ({ commandPaletteOpen: !state.commandPaletteOpen })),
  setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
  setIsDragOver: (isDragOver) => set({ isDragOver }),
}));