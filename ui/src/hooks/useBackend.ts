import { useState, useEffect, useRef } from "react";
import type { HealthResponse } from "../lib/types";

interface BackendState {
  health: HealthResponse | null;
  connected: boolean;
  port: number;
}

const BACKEND_PORT = 8080;
const POLL_INTERVAL_MS = 1500;

export function useBackend(): BackendState {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [connected, setConnected] = useState(false);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;

    const poll = async () => {
      try {
        const resp = await fetch(`http://127.0.0.1:${BACKEND_PORT}/health`, {
          signal: AbortSignal.timeout(3000),
        });
        if (!mountedRef.current) return;
        if (resp.ok) {
          const data: HealthResponse = await resp.json();
          if (!mountedRef.current) return;
          setHealth(data);
          setConnected(true);
        } else {
          setConnected(false);
        }
      } catch {
        if (!mountedRef.current) return;
        setConnected(false);
      }
    };

    // Immediate first attempt
    poll();

    // Then poll at interval
    const interval = setInterval(poll, POLL_INTERVAL_MS);

    return () => {
      mountedRef.current = false;
      clearInterval(interval);
    };
  }, []);

  return { health, connected, port: BACKEND_PORT };
}