import { useState, useCallback, useEffect, useRef } from "react";
import type { Terminal } from "@xterm/xterm";
import { SearchAddon } from "@xterm/addon-search";

interface TerminalSearchProps {
  terminal: Terminal | null;
  visible: boolean;
  onClose: () => void;
}

export function TerminalSearch({ terminal, visible, onClose }: TerminalSearchProps) {
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const searchAddonRef = useRef<SearchAddon | null>(null);

  useEffect(() => {
    if (!terminal) return;
    const addon = new SearchAddon();
    terminal.loadAddon(addon);
    searchAddonRef.current = addon;
    return () => {
      addon.dispose();
      searchAddonRef.current = null;
    };
  }, [terminal]);

  useEffect(() => {
    if (visible && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [visible]);

  const handleSearch = useCallback(
    (direction: "next" | "prev") => {
      if (!searchAddonRef.current || !query) return;
      if (direction === "next") {
        searchAddonRef.current.findNext(query);
      } else {
        searchAddonRef.current.findPrevious(query);
      }
    },
    [query],
  );

  useEffect(() => {
    if (!visible) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        setQuery("");
        searchAddonRef.current?.clearDecorations();
      }
      if (e.key === "Enter") {
        e.preventDefault();
        handleSearch(e.shiftKey ? "prev" : "next");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [visible, onClose, handleSearch]);

  if (!visible) return null;

  return (
    <div className="absolute top-2 right-2 z-20 flex items-center gap-1 bg-[#1a1a2e] border border-[#2a2a3e] rounded-lg px-2 py-1 shadow-xl">
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search..."
        className="w-40 bg-transparent text-xs text-[#DCDCDC] placeholder-[#555] focus:outline-none"
      />
      <button
        onClick={() => handleSearch("prev")}
        className="text-[#7A7A7A] hover:text-[#DCDCDC] text-xs px-1"
        title="Previous (Shift+Enter)"
      >
        ▲
      </button>
      <button
        onClick={() => handleSearch("next")}
        className="text-[#7A7A7A] hover:text-[#DCDCDC] text-xs px-1"
        title="Next (Enter)"
      >
        ▼
      </button>
      <button
        onClick={() => {
          onClose();
          setQuery("");
          searchAddonRef.current?.clearDecorations();
        }}
        className="text-[#7A7A7A] hover:text-[#DCDCDC] text-xs px-1"
        title="Close (Esc)"
      >
        ✕
      </button>
    </div>
  );
}