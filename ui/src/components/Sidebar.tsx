import type { ProjectInfo, ProviderAccountInfo } from "../lib/types";

interface SidebarProps {
  projects: ProjectInfo[];
  accounts: ProviderAccountInfo[];
  activeProjectId: string | null;
  onProjectSelect: (id: string | null) => void;
}

export function Sidebar({ projects, accounts, activeProjectId, onProjectSelect }: SidebarProps) {
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
          <div className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2 px-1">
            Contas
          </div>
          {accounts.length === 0 ? (
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