import { Command } from "cmdk";
import { useEffect } from "react";
import { useAppStore } from "../stores/appStore";

interface CommandPaletteProps {
  onSpawnSession?: (agentBinary: string) => void;
  onImportProject?: () => void;
}

export function CommandPalette({ onSpawnSession, onImportProject }: CommandPaletteProps) {
  const { commandPaletteOpen, setCommandPaletteOpen, agents, projects, setActiveProjectId, setLayout, setActiveView } = useAppStore();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setCommandPaletteOpen(!commandPaletteOpen);
      }
      if (e.key === "Escape" && commandPaletteOpen) {
        setCommandPaletteOpen(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [commandPaletteOpen, setCommandPaletteOpen]);

  if (!commandPaletteOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh] bg-black/60 backdrop-blur-sm">
      <Command
        className="w-full max-w-lg bg-[#121212] border border-[#232323] rounded-xl shadow-2xl overflow-hidden"
        label="Command Palette"
      >
        <Command.Input
          placeholder="Type a command or search..."
          className="w-full px-4 py-3 bg-transparent text-[#DCDCDC] text-sm placeholder-[#555] focus:outline-none border-b border-[#232323]"
          autoFocus
        />
        <Command.List className="max-h-80 overflow-y-auto p-2">
          <Command.Empty className="px-3 py-4 text-center text-[#555] text-xs">
            No results found
          </Command.Empty>

          <Command.Group heading="Actions" className="mb-2">
            <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-[#555]">
              Actions
            </div>
            {agents.map((agent) => (
              <Command.Item
                key={agent.binary}
                onSelect={() => {
                  onSpawnSession?.(agent.binary);
                  setCommandPaletteOpen(false);
                }}
                className="px-3 py-2 rounded-md text-[12px] text-[#C9C9C9] cursor-pointer hover:bg-[#1A1A1A] data-[selected=true]:bg-[#1A1A1A] flex items-center gap-2"
              >
                <span className="text-[#4ADE80]">▶</span>
                Spawn {agent.name} session
              </Command.Item>
            ))}
            <Command.Item
              onSelect={() => {
                onImportProject?.();
                setCommandPaletteOpen(false);
              }}
              className="px-3 py-2 rounded-md text-[12px] text-[#C9C9C9] cursor-pointer hover:bg-[#1A1A1A] data-[selected=true]:bg-[#1A1A1A] flex items-center gap-2"
            >
              <span className="text-[#007acc]">📁</span>
              Import project
            </Command.Item>
          </Command.Group>

          <Command.Group heading="Layout" className="mb-2">
            <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-[#555]">
              Layout
            </div>
            {(["grid", "spotlight", "sidebar"] as const).map((mode) => (
              <Command.Item
                key={mode}
                onSelect={() => {
                  setLayout(mode);
                  setCommandPaletteOpen(false);
                }}
                className="px-3 py-2 rounded-md text-[12px] text-[#C9C9C9] cursor-pointer hover:bg-[#1A1A1A] data-[selected=true]:bg-[#1A1A1A] flex items-center gap-2"
              >
                <span className="text-[#A3A3A3]">⊞</span>
                Switch to {mode} layout
              </Command.Item>
            ))}
          </Command.Group>

          {projects.length > 0 && (
            <Command.Group heading="Projects">
              <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-[#555]">
                Projects
              </div>
              {projects.map((project) => (
                <Command.Item
                  key={project.id}
                  onSelect={() => {
                    setActiveProjectId(project.id);
                    setCommandPaletteOpen(false);
                  }}
                  className="px-3 py-2 rounded-md text-[12px] text-[#C9C9C9] cursor-pointer hover:bg-[#1A1A1A] data-[selected=true]:bg-[#1A1A1A] flex items-center gap-2"
                >
                  <span className="text-[#F97316]">◉</span>
                  {project.name}
                </Command.Item>
              ))}
            </Command.Group>
          )}

          <Command.Group heading="Views">
            <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-[#555]">
              Views
            </div>
            <Command.Item
              onSelect={() => {
                setActiveView("terminals");
                setCommandPaletteOpen(false);
              }}
              className="px-3 py-2 rounded-md text-[12px] text-[#C9C9C9] cursor-pointer hover:bg-[#1A1A1A] data-[selected=true]:bg-[#1A1A1A] flex items-center gap-2"
            >
              <span className="text-[#A3A3A3]">⬛</span>
              Terminals
            </Command.Item>
            <Command.Item
              onSelect={() => {
                setActiveView("orchestrator");
                setCommandPaletteOpen(false);
              }}
              className="px-3 py-2 rounded-md text-[12px] text-[#C9C9C9] cursor-pointer hover:bg-[#1A1A1A] data-[selected=true]:bg-[#1A1A1A] flex items-center gap-2"
            >
              <span className="text-[#A3A3A3]">🎯</span>
              Orchestrator
            </Command.Item>
          </Command.Group>
        </Command.List>

        <div className="px-3 py-2 border-t border-[#232323] flex items-center gap-3 text-[10px] text-[#555]">
          <span>↑↓ navigate</span>
          <span>↵ select</span>
          <span>esc close</span>
        </div>
      </Command>
    </div>
  );
}