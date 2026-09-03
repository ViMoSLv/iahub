import { useState } from "react";
import type { LayoutMode } from "../lib/types";

interface AgentInfo {
  name: string;
  binary: string;
  status: string;
}

interface WorkspaceTab {
  id: string;
  name: string;
  icon: string;
  count: number;
  color: string;
}

interface HeaderProps {
  layout: LayoutMode;
  onLayoutChange: (mode: LayoutMode) => void;
  sessionCount: number;
  connected: boolean;
  agents: AgentInfo[];
  onSpawnSession: (agentBinary: string, accountId?: string) => void;
}

const DEMO_TABS: WorkspaceTab[] = [
  { id: "barbearia", name: "Barbearia", icon: "🟠", count: 2, color: "#F97316" },
  { id: "discord", name: "Discord", icon: "💬", count: 1, color: "#5865F2" },
  { id: "loja", name: "Loja", icon: "🛒", count: 3, color: "#EAB308" },
];

export function Header({ layout, onLayoutChange, sessionCount, connected, agents, onSpawnSession }: HeaderProps) {
  const [showSpawnMenu, setShowSpawnMenu] = useState(false);
  const [activeTab, setActiveTab] = useState("barbearia");

  return (
    <header className="h-10 bg-[#0B0B0B] border-b border-[#171717] flex items-center px-3 gap-1 shrink-0 select-none">
      {/* Hamburger */}
      <button className="w-7 h-7 flex items-center justify-center rounded hover:bg-[#1A1A1A] transition-colors text-[#7A7A7A] hover:text-[#C9C9C9]">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M1.5 3.5h11M1.5 7h11M1.5 10.5h11" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round"/>
        </svg>
      </button>

      {/* App Name */}
      <span className="text-[13px] font-semibold text-[#DCDCDC] mr-2 tracking-tight">IA-Hub</span>

      {/* Workspace Tabs */}
      <div className="flex items-center gap-0.5">
        {DEMO_TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[12px] transition-all ${
              activeTab === tab.id
                ? "bg-[#1A1A1A] text-[#DCDCDC] border border-[#2A2A2A]"
                : "text-[#7A7A7A] hover:text-[#C9C9C9] hover:bg-[#141414] border border-transparent"
            }`}
          >
            <span className="text-[11px]">{tab.icon}</span>
            <span className="font-medium">{tab.name}</span>
            <span
              className="text-[10px] font-bold min-w-[16px] h-[16px] flex items-center justify-center rounded-full px-1"
              style={{
                backgroundColor: activeTab === tab.id ? `${tab.color}25` : "#1A1A1A",
                color: activeTab === tab.id ? tab.color : "#555",
              }}
            >
              {tab.count}
            </span>
          </button>
        ))}
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Spawn session */}
      <div className="relative mr-1">
        <button
          onClick={() => setShowSpawnMenu(!showSpawnMenu)}
          className="flex items-center gap-1 px-2 py-1 rounded-md text-[11px] font-medium text-[#7A7A7A] hover:text-[#C9C9C9] hover:bg-[#1A1A1A] transition-colors border border-transparent hover:border-[#2A2A2A]"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M6 2v8M2 6h8" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round"/>
          </svg>
          Nova Sessão
        </button>
        {showSpawnMenu && (
          <>
            <div className="fixed inset-0 z-40" onClick={() => setShowSpawnMenu(false)} />
            <div className="absolute top-full right-0 mt-1 w-52 bg-[#141414] border border-[#2A2A2A] rounded-lg shadow-2xl z-50 py-1 overflow-hidden">
              {agents.length === 0 ? (
                <div className="px-3 py-2 text-[#555] text-[11px]">Nenhum agente descoberto</div>
              ) : (
                agents.map((agent) => (
                  <button
                    key={agent.binary}
                    onClick={() => {
                      onSpawnSession(agent.binary);
                      setShowSpawnMenu(false);
                    }}
                    disabled={agent.status === "not_found"}
                    className={`w-full text-left px-3 py-1.5 text-[11px] flex items-center gap-2 transition-colors ${
                      agent.status === "not_found"
                        ? "text-[#333] cursor-not-allowed"
                        : "text-[#C9C9C9] hover:bg-[#1E1E1E]"
                    }`}
                  >
                    <span
                      className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                        agent.status === "ok"
                          ? "bg-[#4ADE80]"
                          : agent.status === "version_unknown"
                          ? "bg-[#F0C24B]"
                          : "bg-[#333]"
                      }`}
                    />
                    <span className="flex-1">{agent.name}</span>
                    <span className="text-[#555] text-[10px] font-mono">{agent.binary}</span>
                  </button>
                ))
              )}
            </div>
          </>
        )}
      </div>

      {/* Layout switcher */}
      <div className="flex items-center bg-[#141414] rounded-md p-0.5 border border-[#1E1E1E]">
        {(["grid", "spotlight", "sidebar"] as LayoutMode[]).map((mode) => (
          <button
            key={mode}
            onClick={() => onLayoutChange(mode)}
            className={`px-2 py-0.5 rounded text-[10px] font-medium transition-all ${
              layout === mode
                ? "bg-[#2A2A2A] text-[#DCDCDC] shadow-sm"
                : "text-[#555] hover:text-[#999]"
            }`}
          >
            {mode}
          </button>
        ))}
      </div>

      {/* Memory / Connection indicator */}
      <div className="flex items-center gap-2 ml-1">
        <div className="flex items-center gap-1.5">
          <div
            className={`w-1.5 h-1.5 rounded-full ${
              connected ? "bg-[#4ADE80]" : "bg-[#EF4444]"
            }`}
          />
          <span className="text-[10px] text-[#555] font-mono">
            {connected ? `${sessionCount} sessões` : "offline"}
          </span>
        </div>
      </div>
    </header>
  );
}