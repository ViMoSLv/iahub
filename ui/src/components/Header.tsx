import { useState } from "react";
import type { LayoutMode, ProviderAccountInfo } from "../lib/types";

interface AgentInfo {
  name: string;
  binary: string;
  status: string;
}

interface HeaderProps {
  layout: LayoutMode;
  onLayoutChange: (mode: LayoutMode) => void;
  sessionCount: number;
  connected: boolean;
  agents: AgentInfo[];
  accounts: ProviderAccountInfo[];
  onSpawnSession: (agentBinary: string, accountId?: string) => void;
}

export function Header({ layout, onLayoutChange, sessionCount, connected, agents, accounts, onSpawnSession }: HeaderProps) {
  const [showSpawnMenu, setShowSpawnMenu] = useState(false);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);

  const availableAgents = agents.filter((a) => a.status !== "not_found");
  const availableAccounts = accounts.filter((a) => a.status === "active");

  return (
    <header className="h-[var(--header-height)] bg-[#121212] border-b border-[#171717] flex items-center px-3 gap-3 shrink-0">
      {/* Logo */}
      <div className="flex items-center gap-2">
        <div className="w-5 h-5 rounded-md bg-[#007acc] flex items-center justify-center text-[10px] font-bold text-white">
          IA
        </div>
        <span className="font-semibold text-[13px] text-[#DCDCDC] tracking-tight">IA-Hub</span>
      </div>

      {/* Divider */}
      <div className="w-px h-4 bg-[#232323]" />

      {/* Spawn session button */}
      <div className="relative">
        <button
          onClick={() => { setShowSpawnMenu(!showSpawnMenu); setSelectedAgent(null); }}
          className="px-2.5 py-1 rounded-md bg-[#007acc]/15 text-[#007acc] text-[11px] font-medium hover:bg-[#007acc]/25 transition-colors border border-[#007acc]/20"
        >
          + Nova Sessão
        </button>
        {showSpawnMenu && (
          <div className="absolute top-full left-0 mt-1 w-60 bg-[#121212] border border-[#232323] rounded-lg shadow-2xl z-50 py-1">
            {!selectedAgent ? (
              <>
                <div className="px-3 py-1.5 text-[10px] text-[#7A7A7A] uppercase tracking-wider font-semibold">
                  Escolher Agente
                </div>
                {availableAgents.length === 0 ? (
                  <div className="px-3 py-2 text-[#555] text-xs">Nenhum agente descoberto</div>
                ) : (
                  availableAgents.map((agent) => (
                    <button
                      key={agent.binary}
                      onClick={() => {
                        if (availableAccounts.length > 0) {
                          setSelectedAgent(agent.binary);
                        } else {
                          onSpawnSession(agent.binary);
                          setShowSpawnMenu(false);
                        }
                      }}
                      className="w-full text-left px-3 py-1.5 text-[12px] flex items-center gap-2 transition-colors text-[#C9C9C9] hover:bg-[#1A1A1A]"
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                          agent.status === "ok"
                            ? "bg-[#4ADE80]"
                            : "bg-[#F0C24B]"
                        }`}
                      />
                      <span className="flex-1">{agent.name}</span>
                      <span className="text-[#555] text-[10px]">{agent.binary}</span>
                    </button>
                  ))
                )}
              </>
            ) : (
              <>
                <div className="px-3 py-1.5 text-[10px] text-[#7A7A7A] uppercase tracking-wider font-semibold">
                  Conta para {agents.find(a => a.binary === selectedAgent)?.name}
                </div>
                {availableAccounts.map((account) => (
                  <button
                    key={account.id}
                    onClick={() => {
                      onSpawnSession(selectedAgent, account.id);
                      setShowSpawnMenu(false);
                      setSelectedAgent(null);
                    }}
                    className="w-full text-left px-3 py-1.5 text-[12px] flex items-center gap-2 transition-colors text-[#C9C9C9] hover:bg-[#1A1A1A]"
                  >
                    <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-[#4ADE80]" />
                    <span className="flex-1">{account.label}</span>
                    <span className="text-[#555] text-[10px]">
                      {account.active_sessions}/{account.max_concurrent_sessions}
                    </span>
                  </button>
                ))}
                <button
                  onClick={() => {
                    onSpawnSession(selectedAgent);
                    setShowSpawnMenu(false);
                    setSelectedAgent(null);
                  }}
                  className="w-full text-left px-3 py-1.5 text-[12px] flex items-center gap-2 transition-colors text-[#7A7A7A] hover:bg-[#1A1A1A] border-t border-[#232323] mt-1 pt-2"
                >
                  <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-[#555]" />
                  <span className="flex-1">Sem conta (auto)</span>
                </button>
                <button
                  onClick={() => setSelectedAgent(null)}
                  className="w-full text-left px-3 py-1 text-[10px] text-[#555] hover:text-[#A3A3A3] transition-colors"
                >
                  ← Voltar
                </button>
              </>
            )}
          </div>
        )}
      </div>

      {/* Session count badge */}
      <div className="flex items-center gap-1.5">
        <span className="px-2 py-0.5 rounded-md bg-[#1A1A1A] border border-[#232323] text-[#A3A3A3] text-[11px]">
          {sessionCount} sessão{sessionCount !== 1 ? "es" : ""}
        </span>
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Layout switcher */}
      <div className="flex items-center gap-0.5 bg-[#0B0B0B] rounded-md p-0.5 border border-[#171717]">
        {(["grid", "spotlight", "sidebar"] as LayoutMode[]).map((mode) => (
          <button
            key={mode}
            onClick={() => onLayoutChange(mode)}
            className={`px-2.5 py-0.5 rounded text-[11px] font-medium transition-colors ${
              layout === mode
                ? "bg-[#1A1A1A] text-[#DCDCDC] border border-[#333]"
                : "text-[#7A7A7A] hover:text-[#C9C9C9] border border-transparent"
            }`}
          >
            {mode === "grid" ? "⊞" : mode === "spotlight" ? "◧" : "☰"}
          </button>
        ))}
      </div>

      {/* Connection status */}
      <div className="flex items-center gap-1.5 text-[11px]">
        <div
          className={`w-1.5 h-1.5 rounded-full ${
            connected ? "bg-[#4ADE80]" : "bg-[#f44747]"
          }`}
        />
        <span className="text-[#7A7A7A]">
          {connected ? "connected" : "offline"}
        </span>
      </div>
    </header>
  );
}