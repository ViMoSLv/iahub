interface ActivityBarProps {
  activePanel: "explorer" | "sessions" | "orchestrator" | null;
  onPanelToggle: (panel: "explorer" | "sessions" | "orchestrator") => void;
  hasFolder: boolean;
}

const icons: Record<string, { path: string; label: string }> = {
  explorer: {
    label: "Explorer",
    path: "M2 3h5l1.5 1.5H14v9H2V3zm0 1.5v8h11v-7H8L6.5 4H2z",
  },
  sessions: {
    label: "Sessions",
    path: "M3 2h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3a1 1 0 011-1zm1 2v3h8V4H4zm0 5v3h8V9H4z",
  },
  orchestrator: {
    label: "Orchestrator",
    path: "M8 1a2 2 0 012 2v1h3a1 1 0 011 1v2h-2V5h-2v2a2 2 0 01-4 0V5H4v2H2V5a1 1 0 011-1h3V3a2 2 0 012-2zM4 9h2v2H4V9zm4 0h2v2H8V9zm4 0h2v2h-2V9zM4 12h8v2H4v-2z",
  },
};

export function ActivityBar({ activePanel, onPanelToggle, hasFolder }: ActivityBarProps) {
  return (
    <div className="w-[var(--activity-bar-width)] bg-[#0B0B0B] border-r border-[#171717] flex flex-col items-center py-2 gap-1 shrink-0">
      {/* Top icons */}
      <div className="flex flex-col gap-0.5 w-full">
        {(Object.entries(icons) as [string, { path: string; label: string }][]).map(
          ([key, { path, label }]) => {
            const isActive = activePanel === key;
            return (
              <button
                key={key}
                onClick={() => onPanelToggle(key as "explorer" | "sessions" | "orchestrator")}
                data-tooltip={label}
                className={`activity-icon w-full h-10 flex items-center justify-center rounded-none relative ${
                  isActive
                    ? "text-[#DCDCDC] border-l-2 border-[#007acc] bg-[#121212]"
                    : "text-[#7A7A7A] border-l-2 border-transparent hover:text-[#C9C9C9] hover:bg-[#161616]"
                }`}
                title={label}
              >
                <svg width="20" height="20" viewBox="0 0 16 16" fill="currentColor">
                  <path d={path} />
                </svg>
                {key === "explorer" && hasFolder && (
                  <span className="absolute top-1.5 right-1.5 w-1.5 h-1.5 rounded-full bg-[#4ADE80]" />
                )}
              </button>
            );
          }
        )}
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Bottom icons */}
      <div className="flex flex-col gap-0.5 w-full">
        <button
          data-tooltip="Settings"
          className="activity-icon w-full h-10 flex items-center justify-center text-[#7A7A7A] border-l-2 border-transparent hover:text-[#C9C9C9] hover:bg-[#161616]"
          title="Settings"
        >
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2">
            <circle cx="8" cy="8" r="2.5" />
            <path d="M8 1v2M8 13v2M1 8h2M13 8h2M2.9 2.9l1.4 1.4M11.7 11.7l1.4 1.4M2.9 13.1l1.4-1.4M11.7 4.3l1.4-1.4" />
          </svg>
        </button>
      </div>
    </div>
  );
}