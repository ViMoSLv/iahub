import { useState } from "react";
import type { ProjectInfo, ProviderAccountInfo } from "../lib/types";

interface SidebarProps {
  projects: ProjectInfo[];
  accounts: ProviderAccountInfo[];
  activeProjectId: string | null;
  onProjectSelect: (id: string | null) => void;
  onAddAccount?: (provider: string, label: string) => void;
}

// Demo agent assignments per project for visual matching with reference image
const PROJECT_AGENTS: Record<string, Array<{ name: string; color: string; status: "active" | "idle" }>> = {
  "Barbearia": [{ name: "claude", color: "#F97316", status: "active" }],
  "Discord Server Manager": [{ name: "claude", color: "#4ADE80", status: "active" }],
  "Loja Online": [{ name: "codex", color: "#F0C24B", status: "idle" }],
  "Portfolio": [{ name: "claude", color: "#6E6E6E", status: "idle" }],
};

export function Sidebar({ projects, accounts, activeProjectId, onProjectSelect, onAddAccount }: SidebarProps) {
  const [showAddForm, setShowAddForm] = useState(false);
  const [newProvider, setNewProvider] = useState("claude");
  const [newLabel, setNewLabel] = useState("");
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>({
    PRODUTOS: true,
    PESSOAL: true,
  });

  const handleAdd = () => {
    if (!newLabel.trim() || !onAddAccount) return;
    onAddAccount(newProvider, newLabel.trim());
    setNewLabel("");
    setShowAddForm(false);
  };

  const toggleGroup = (group: string) => {
    setExpandedGroups((prev) => ({ ...prev, [group]: !prev[group] }));
  };

  // Split projects into categories for visual grouping like the reference
  const produtos = projects.filter((p) =>
    ["Barbearia", "Discord Server Manager", "Loja Online"].includes(p.name)
  );
  const pessoal = projects.filter((p) =>
    ["Portfolio"].includes(p.name)
  );
  const outros = projects.filter(
    (p) => !["Barbearia", "Discord Server Manager", "Loja Online", "Portfolio"].includes(p.name)
  );

  const renderProjectItem = (project: ProjectInfo) => {
    const agents = PROJECT_AGENTS[project.name] || [];
    const isActive = activeProjectId === project.id;

    return (
      <div key={project.id}>
        <button
          onClick={() => onProjectSelect(isActive ? null : project.id)}
          className={`w-full text-left px-2 py-1.5 rounded-md text-[12px] transition-colors flex items-center gap-2 ${
            isActive
              ? "bg-[#1A1A1A] text-[#DCDCDC] border border-[#232323]"
              : "text-[#C9C9C9] hover:bg-[#161616] border border-transparent"
          }`}
        >
          <span
            className="w-3 h-3 rounded-sm shrink-0 flex items-center justify-center text-[8px]"
            style={{ backgroundColor: `${agents[0]?.color || "#555"}20`, color: agents[0]?.color || "#555" }}
          >
            {agents[0]?.name === "codex" ? "⬡" : "◉"}
          </span>
          <span className="truncate flex-1 font-medium">{project.name}</span>
          <span className="text-[10px] text-[#555] bg-[#121212] px-1.5 py-0.5 rounded-full border border-[#232323] min-w-[18px] text-center">
            {agents.length || 0}
          </span>
        </button>
        {/* Agent sub-items */}
        {agents.map((agent, i) => (
          <div
            key={i}
            className="ml-5 px-2 py-1 flex items-center gap-2 text-[11px] text-[#7A7A7A] hover:text-[#C9C9C9] transition-colors cursor-pointer rounded hover:bg-[#141414]"
          >
            <span
              className="w-3.5 h-3.5 rounded-sm flex items-center justify-center text-[7px] font-bold"
              style={{ backgroundColor: `${agent.color}15`, color: agent.color }}
            >
              {agent.name === "codex" ? "⬡" : "▣"}
            </span>
            <span className="flex-1">{agent.name}</span>
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                agent.status === "active" ? "bg-[#4ADE80]" : "bg-[#555]"
              }`}
            />
          </div>
        ))}
      </div>
    );
  };

  const renderGroup = (label: string, items: ProjectInfo[], suspended?: boolean) => {
    if (items.length === 0) return null;
    const expanded = expandedGroups[label] !== false;

    return (
      <div className="mb-3">
        <button
          onClick={() => toggleGroup(label)}
          className="w-full flex items-center gap-1.5 px-1 py-1 text-[10px] font-semibold uppercase tracking-wider text-[#555] hover:text-[#7A7A7A] transition-colors"
        >
          <svg
            width="8"
            height="8"
            viewBox="0 0 8 8"
            fill="none"
            className={`transition-transform ${expanded ? "rotate-90" : ""}`}
          >
            <path d="M2 1l4 3-4 3V1z" fill="currentColor" />
          </svg>
          <span className="flex items-center gap-1.5">
            <span
              className="w-1.5 h-1.5 rounded-full"
              style={{ backgroundColor: suspended ? "#6E6E6E" : "#5865F2" }}
            />
            {label}
            {suspended && (
              <span className="text-[9px] text-[#555] font-normal normal-case tracking-normal">
                (SUSPENSO)
              </span>
            )}
          </span>
        </button>
        {expanded && (
          <div className="space-y-0.5 mt-0.5">
            {items.map(renderProjectItem)}
          </div>
        )}
      </div>
    );
  };

  return (
    <aside className="h-full bg-[#0B0B0B] flex flex-col shrink-0 overflow-hidden">
      {/* Header */}
      <div className="h-9 flex items-center px-3 border-b border-[#171717] shrink-0">
        <span className="text-[11px] font-semibold uppercase tracking-wider text-[#7A7A7A]">
          Projetos
        </span>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {projects.length === 0 ? (
          <div className="px-2 py-6 text-center">
            <div className="text-[#555] text-[11px] mb-2">Nenhum projeto importado</div>
            <button className="text-[#007acc] text-[11px] hover:underline">
              + Importar repositório
            </button>
          </div>
        ) : (
          <>
            {renderGroup("PRODUTOS", produtos)}
            {renderGroup("PESSOAL", pessoal, true)}
            {outros.length > 0 && renderGroup("OUTROS", outros)}
          </>
        )}

        {/* Accounts section */}
        <div className="mt-4 pt-3 border-t border-[#171717]">
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

      {/* Footer — ORG indicator + user */}
      <div className="border-t border-[#171717]">
        <div className="px-3 py-1.5 flex items-center gap-2">
          <span className="text-[9px] font-semibold uppercase tracking-wider text-[#555]">ORG</span>
          <div className="flex-1 flex items-center gap-1">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="w-3 h-1.5 rounded-sm bg-[#2A2A2A]" />
            ))}
          </div>
          <div className="w-4 h-4 rounded border border-[#2A2A2A] flex items-center justify-center">
            <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
              <rect x="1" y="1" width="2.5" height="2.5" rx="0.5" fill="#555" />
              <rect x="4.5" y="1" width="2.5" height="2.5" rx="0.5" fill="#555" />
              <rect x="1" y="4.5" width="2.5" height="2.5" rx="0.5" fill="#555" />
              <rect x="4.5" y="4.5" width="2.5" height="2.5" rx="0.5" fill="#555" />
            </svg>
          </div>
        </div>
        <div className="px-3 py-2 flex items-center gap-2">
          <div className="w-6 h-6 rounded-full bg-gradient-to-br from-[#F97316] to-[#EAB308] flex items-center justify-center text-[9px] text-white font-bold">
            K
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-[11px] text-[#C9C9C9] truncate font-medium">Kauã Miguel</div>
            <div className="text-[9px] text-[#555]">conta local</div>
          </div>
        </div>
      </div>
    </aside>
  );
}