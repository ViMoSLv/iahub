import { useEffect, useRef } from "react";

const LAYOUT_STORAGE_KEY = "iahub-layout-state";

export interface PersistedLayoutState {
  layout: string;
  panelOrder: string[];
  panelSizes: Record<string, number[]>;
  activeView: string;
  showLogs: boolean;
}

const DEBOUNCE_MS = 500;

export function useLayoutPersistence(
  layout: string,
  panelOrder: string[],
  panelSizes: Record<string, number[]>,
  activeView: string,
  showLogs: boolean,
  setLayout: (l: any) => void,
  setPanelOrder: (o: string[]) => void,
  setPanelSizes: (s: Record<string, number[]>) => void,
  setActiveView: (v: any) => void,
  setShowLogs: (v: boolean) => void,
) {
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const initializedRef = useRef(false);

  // Restore on mount
  useEffect(() => {
    try {
      const raw = localStorage.getItem(LAYOUT_STORAGE_KEY);
      if (!raw) return;
      const saved: PersistedLayoutState = JSON.parse(raw);
      if (saved.layout) setLayout(saved.layout);
      if (saved.panelOrder?.length) setPanelOrder(saved.panelOrder);
      if (saved.panelSizes) setPanelSizes(saved.panelSizes);
      if (saved.activeView) setActiveView(saved.activeView);
      if (typeof saved.showLogs === "boolean") setShowLogs(saved.showLogs);
    } catch {
      // Corrupted storage — ignore and use defaults
    }
    initializedRef.current = true;
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Save on change (debounced)
  useEffect(() => {
    if (!initializedRef.current) return;
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      try {
        const state: PersistedLayoutState = {
          layout,
          panelOrder,
          panelSizes,
          activeView,
          showLogs,
        };
        localStorage.setItem(LAYOUT_STORAGE_KEY, JSON.stringify(state));
      } catch {
        // Storage full or unavailable — ignore
      }
    }, DEBOUNCE_MS);
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
  }, [layout, panelOrder, panelSizes, activeView, showLogs]);
}

/** Clear persisted layout (e.g., for a "Reset Layout" action) */
export function clearPersistedLayout() {
  try {
    localStorage.removeItem(LAYOUT_STORAGE_KEY);
  } catch {
    // ignore
  }
}