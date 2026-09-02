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
    <header className="h-[var(--header-height)] bg-surface-raised border-b border-[var(--border-color)] flex items-center px-4 gap-4 shrink-0">
      {/* Logo */}
      <div className="flex items-center gap-2">
        <div className="w-6 h-6 rounded bg-accent flex items-center justify-center text-xs font-bold text-white">
          IA
        </div>
        <span className="font-bold text-sm text-gray-200">IA-Hub</span>
      </div>

      {/* Spawn session button */}
      <div className="relative">
        <button
          onClick={() => { setShowSpawnMenu(!showSpawnMenu); setSelectedAgent(null); }}
          className="px-3 py-1 rounded-lg bg-accent/20 text-accent text-xs font-medium hover:bg-accent/30 transition-colors"
        >
          + Nova Sessão
        </button>
        {showSpawnMenu && (
          <div className="absolute top-full left-0 mt-1 w-64 bg-surface-raised border border-[var(--border-color)] rounded-lg shadow-xl z-50 py-1">
            {!selectedAgent ? (
              /* Step 1: Choose agent */
              <>
                <div className="px-3 py-1.5 text-[10px] text-gray-500 uppercase tracking-wider font-semibold">
                  Escolher Agente
                </div>
                {availableAgents.length === 0 ? (
                  <div className="px-3 py-2 text-gray-500 text-xs">Nenhum agente descoberto</div>
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
                      className="w-full text-left px-3 py-2 text-xs flex items-center gap-2 transition-colors text-gray-300 hover:bg-white/5"
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                          agent.status === "ok"
                            ? "bg-status-success"
                            : "bg-yellow-500"
                        }`}
                      />
                      <span className="flex-1">{agent.name}</span>
                      <span className="text-gray-500 text-[10px]">{agent.binary}</span>
                    </button>
                  ))
                )}
              </>
            ) : (
              /* Step 2: Choose account */
              <>
                <div className="px-3 py-1.5 text-[10px] text-gray-500 uppercase tracking-wider font-semibold">
                  Escolher Conta para {agents.find(a => a.binary === selectedAgent)?.name}
                </div>
                {availableAccounts.map((account) => (
                  <button
                    key={account.id}
                    onClick={() => {
                      onSpawnSession(selectedAgent, account.id);
                      setShowSpawnMenu(false);
                      setSelectedAgent(null);
                    }}
                    className="w-full text-left px-3 py-2 text-xs flex items-center gap-2 transition-colors text-gray-300 hover:bg-white/5"
                  >
                    <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-status-success" />
                    <span className="flex-1">{account.label}</span>
                    <span className="text-gray-500 text-[10px]">
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
                  className="w-full text-left px-3 py-2 text-xs flex items-center gap-2 transition-colors text-gray-400 hover:bg-white/5 border-t border-[var(--border-color)] mt-1 pt-2"
                >
                  <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-gray-500" />
                  <span className="flex-1">Sem conta (isolamento automático)</span>
                </button>
                <button
                  onClick={() => setSelectedAgent(null)}
                  className="w-full text-left px-3 py-1.5 text-[10px] text-gray-500 hover:text-gray-300 transition-colors"
                >
                  ← Voltar
                </button>
              </>
            )}
          </div>
        )}
      </div>

      {/* Workspace tabs placeholder */}
      <div className="flex items-center gap-2 ml-2">
        <span className="px-3 py-1 rounded-full bg-accent/20 text-accent text-xs font-medium">
          Workspace <span className="ml-1 opacity-70">{sessionCount}</span>
        </span>
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Layout switcher */}
      <div className="flex items-center gap-1 bg-surface rounded-lg p-0.5">
        {(["sidebar", "spotlight", "grid"] as LayoutMode[]).map((mode) => (
          <button
            key={mode}
            onClick={() => onLayoutChange(mode)}
            className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
              layout === mode
                ? "bg-accent/20 text-accent"
                : "text-gray-400 hover:text-gray-200"
            }`}
          >
            {mode}
          </button>
        ))}
      </div>

      {/* Connection status */}
      <div className="flex items-center gap-2 text-xs">
        <div
          className={`w-2 h-2 rounded-full ${
            connected ? "bg-status-success" : "bg-status-error"
          }`}
        />
        <span className="text-gray-400">
          {connected ? `${sessionCount} active` : "disconnected"}
        </span>
      </div>
    </header>
  );
}