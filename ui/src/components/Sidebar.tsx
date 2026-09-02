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
    <aside className="w-[var(--sidebar-width)] bg-surface-raised border-r border-[var(--border-color)] flex flex-col shrink-0 overflow-hidden">
      {/* Projects section */}
      <div className="flex-1 overflow-y-auto p-3">
        <div className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2 px-1">
          Projetos
        </div>

        {projects.length === 0 ? (
          <div className="px-2 py-4 text-center">
            <div className="text-gray-500 text-xs mb-2">Nenhum projeto importado</div>
            <button className="text-accent text-xs hover:underline">
              + Importar repositório
            </button>
          </div>
        ) : (
          <div className="space-y-1">
            {projects.map((project) => (
              <button
                key={project.id}
                onClick={() => onProjectSelect(project.id === activeProjectId ? null : project.id)}
                className={`w-full text-left px-2 py-1.5 rounded-md text-sm transition-colors flex items-center gap-2 ${
                  activeProjectId === project.id
                    ? "bg-accent/10 text-accent"
                    : "text-gray-300 hover:bg-white/5"
                }`}
              >
                <span className="w-2 h-2 rounded-full bg-status-active shrink-0" />
                <span className="truncate flex-1">{project.name}</span>
                {project.session_count > 0 && (
                  <span className="text-xs text-gray-500 bg-surface px-1.5 py-0.5 rounded-full">
                    {project.session_count}
                  </span>
                )}
              </button>
            ))}
          </div>
        )}

        {/* Accounts section */}
        <div className="mt-6">
          <div className="flex items-center justify-between mb-2 px-1">
            <div className="text-xs font-semibold text-gray-400 uppercase tracking-wider">
              Contas
            </div>
            {onAddAccount && (
              <button
                onClick={() => setShowAddForm(!showAddForm)}
                className="text-accent text-xs hover:text-accent/80 transition-colors"
              >
                + Add
              </button>
            )}
          </div>

          {/* Add account form */}
          {showAddForm && (
            <div className="px-2 py-2 mb-2 bg-surface rounded-lg border border-[var(--border-color)] space-y-2">
              <select
                value={newProvider}
                onChange={(e) => setNewProvider(e.target.value)}
                className="w-full px-2 py-1 bg-surface-raised border border-[var(--border-color)] rounded text-xs text-gray-200 focus:outline-none focus:border-accent"
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
                className="w-full px-2 py-1 bg-surface-raised border border-[var(--border-color)] rounded text-xs text-gray-200 placeholder-gray-600 focus:outline-none focus:border-accent"
                onKeyDown={(e) => e.key === "Enter" && handleAdd()}
              />
              <div className="flex gap-1">
                <button
                  onClick={handleAdd}
                  disabled={!newLabel.trim()}
                  className="flex-1 py-1 bg-accent/20 text-accent rounded text-xs hover:bg-accent/30 disabled:opacity-50 transition-colors"
                >
                  Salvar
                </button>
                <button
                  onClick={() => setShowAddForm(false)}
                  className="flex-1 py-1 text-gray-500 rounded text-xs hover:text-gray-300 transition-colors"
                >
                  Cancelar
                </button>
              </div>
            </div>
          )}

          {accounts.length === 0 && !showAddForm ? (
            <div className="px-2 py-2 text-gray-500 text-xs">
              Nenhuma conta configurada
            </div>
          ) : (
            <div className="space-y-1">
              {accounts.map((account) => (
                <div
                  key={account.id}
                  className="px-2 py-1.5 rounded-md text-xs flex items-center gap-2"
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                      account.status === "active"
                        ? "bg-status-success"
                        : account.status === "rate_limited"
                        ? "bg-yellow-500"
                        : "bg-status-idle"
                    }`}
                  />
                  <span className="text-gray-300 truncate flex-1">{account.label}</span>
                  <span className="text-gray-500">
                    {account.active_sessions}/{account.max_concurrent_sessions}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="p-3 border-t border-[var(--border-color)]">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 rounded-full bg-accent/20 flex items-center justify-center text-xs text-accent font-bold">
            U
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-xs text-gray-300 truncate">Usuário</div>
            <div className="text-[10px] text-gray-500">conta local</div>
          </div>
        </div>
      </div>
    </aside>
  );
}