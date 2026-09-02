import type { LayoutMode } from "../lib/types";

interface HeaderProps {
  layout: LayoutMode;
  onLayoutChange: (mode: LayoutMode) => void;
  sessionCount: number;
  connected: boolean;
}

export function Header({ layout, onLayoutChange, sessionCount, connected }: HeaderProps) {
  return (
    <header className="h-[var(--header-height)] bg-surface-raised border-b border-[var(--border-color)] flex items-center px-4 gap-4 shrink-0">
      {/* Logo */}
      <div className="flex items-center gap-2">
        <div className="w-6 h-6 rounded bg-accent flex items-center justify-center text-xs font-bold text-white">
          IA
        </div>
        <span className="font-bold text-sm text-gray-200">IA-Hub</span>
      </div>

      {/* Workspace tabs placeholder */}
      <div className="flex items-center gap-2 ml-4">
        <span className="px-3 py-1 rounded-full bg-accent/20 text-accent text-xs font-medium">
          Barbearia <span className="ml-1 opacity-70">{sessionCount}</span>
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