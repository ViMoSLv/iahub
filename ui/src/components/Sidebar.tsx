import { useState } from "react";
import type { ProjectInfo, ProviderAccountInfo } from "../lib/types";

interface SidebarProps {
  projects: ProjectInfo[];
  accounts: ProviderAccountInfo[];
  activeProjectId: string | null;
  onProjectSelect: (id: string | null) => void;
  onAddAccount?: (provider: string, label: string) => void;
}

export function Sidebar({ projects, accounts, activeProjectId, onProjectSelect, onAddAccount }: SidebarProps) {
  const [showAddForm, setShowAddForm] = useState(false);
  const [newProvider, setNewProvider] = useState("claude");
  const [newLabel, setNewLabel] = useState("");

  const handleAdd = () => {
    if (!newLabel.trim() || !onAddAccount) return;
    onAddAccount(newProvider, newLabel.trim());
    setNewLabel("");
    setShowAddForm(false);
  };

  return (
    <aside className="h-full bg-[#0B0B0B] flex flex-col shrink-0 overflow-hidden">
      {/* Header */}
      <div className="h-9 flex items-center px-3 border-b border-[#171717] shrink-0">
        <span className="text-[11px] font-semibold uppercase tracking-wider text-[#7A7A7A]">
          Sessions
        </span>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {/* Projects section */}
        <div className="mb-4">
          <div className="text-[10px] font-semibold text-[#555] uppercase tracking-wider mb-1.5 px-1">
            Projetos
          </div>

          {projects.length === 0 ? (
            <div className="px-2 py-3 text-center">
              <div className="text-[#555] text-[11px] mb-1.5">Nenhum projeto importado</div>
              <button className="text-[#007acc] text-[11px] hover:underline">
                + Importar repositório
              </button>
            </div>
          ) : (
            <div className="space-y-0.5">
              {projects.map((project) => (
                <button
                  key={project.id}
                  onClick={() => onProjectSelect(project.id === activeProjectId ? null : project.id)}
                  className={`w-full text-left px-2 py-1.5 rounded-md text-[12px] transition-colors flex items-center gap-2 ${
                    activeProjectId === project.id
                      ? "bg-[#1A1A1A] text-[#DCDCDC] border border-[#232323]"
                      : "text-[#C9C9C9] hover:bg-[#161616] border border-transparent"
                  }`}
                >
                  <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                    activeProjectId === project.id ? "bg-[#007acc]" : "bg-[#555]"
                  }`} />
                  <span className="truncate flex-1">{project.name}</span>
                  {project.session_count > 0 && (
                    <span className="text-[10px] text-[#555] bg-[#121212] px-1.5 py-0.5 rounded-full border border-[#232323]">
                      {project.session_count}
                    </span>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Accounts section */}
        <div>
          <div className="flex items-center justify-between mb-1.5 px-1">
            <div className="text-[10px] font-semibold text-[#555] uppercase tracking-wider">
              Contas
            </div>
            {onAddAccount && (
              <button
                onClick={() => setShowAddForm(!showAddForm)}
                className="text-[#007acc] text-[10px] hover:text-[#007acc]/80 transition-colors font-medium"
              >
                + Add
              </button>
            )}
          </div>

          {/* Add account form */}
          {showAddForm && (
            <div className="px-2 py-2 mb-2 bg-[#121212] rounded-lg border border-[#232323] space-y-1.5">
              <select
                value={newProvider}
                onChange={(e) => setNewProvider(e.target.value)}
                className="w-full px-2 py-1 bg-[#0B0B0B] border border-[#232323] rounded text-[11px] text-[#C9C9C9] focus:outline-none focus:border-[#007acc]"
              >
                <option value="claude">Claude Code</option>
                <option value="antigravity">Antigravity</option>
                <option value="codex">Codex</option>
                <option value="opencode">OpenCode</option>
              </select>
              <input
                type="text"
                value={newLabel}
                onChange={(e) => setNewLabel(e.target.value)}
                placeholder="Minha conta A"
                className="w-full px-2 py-1 bg-[#0B0B0B] border border-[#232323] rounded text-[11px] text-[#C9C9C9] placeholder-[#555] focus:outline-none focus:border-[#007acc]"
                onKeyDown={(e) => e.key === "Enter" && handleAdd()}
              />
              <div className="flex gap-1">
                <button
                  onClick={handleAdd}
                  disabled={!newLabel.trim()}
                  className="flex-1 py-1 bg-[#007acc]/15 text-[#007acc] rounded text-[11px] hover:bg-[#007acc]/25 disabled:opacity-40 transition-colors border border-[#007acc]/20"
                >
                  Salvar
                </button>
                <button
                  onClick={() => setShowAddForm(false)}
                  className="flex-1 py-1 text-[#7A7A7A] rounded text-[11px] hover:text-[#C9C9C9] hover:bg-[#1A1A1A] transition-colors"
                >
                  Cancelar
                </button>
              </div>
            </div>
          )}

          {accounts.length === 0 && !showAddForm ? (
            <div className="px-2 py-2 text-[#555] text-[11px]">
              Nenhuma conta configurada
            </div>
          ) : (
            <div className="space-y-0.5">
              {accounts.map((account) => (
                <div
                  key={account.id}
                  className="px-2 py-1.5 rounded-md text-[11px] flex items-center gap-2 hover:bg-[#161616] transition-colors"
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                      account.status === "active"
                        ? "bg-[#4ADE80]"
                        : account.status === "rate_limited"
                        ? "bg-[#F0C24B]"
                        : "bg-[#555]"
                    }`}
                  />
                  <span className="text-[#C9C9C9] truncate flex-1">{account.label}</span>
                  <span className="text-[#555] text-[10px]">
                    {account.active_sessions}/{account.max_concurrent_sessions}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="p-2 border-t border-[#171717]">
        <div className="flex items-center gap-2 px-1">
          <div className="w-5 h-5 rounded-md bg-[#007acc]/15 flex items-center justify-center text-[9px] text-[#007acc] font-bold border border-[#007acc]/20">
            U
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-[11px] text-[#C9C9C9] truncate">Usuário</div>
            <div className="text-[9px] text-[#555]">conta local</div>
          </div>
        </div>
      </div>
    </aside>
  );
}