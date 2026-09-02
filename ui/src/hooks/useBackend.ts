import { useState, useEffect, useCallback, useRef } from "react";
import type { HealthResponse } from "../lib/types";

interface BackendState {
  health: HealthResponse | null;
  connected: boolean;
  port: number | null;
  connect: (port: number) => void;
  disconnect: () => void;
}

export function useBackend(): BackendState {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [connected, setConnected] = useState(false);
  const [port, setPort] = useState<number | null>(8080);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const connect = useCallback((p: number) => {
    setPort(p);
    setConnected(true);
  }, []);

  const disconnect = useCallback(() => {
    setConnected(false);
    setHealth(null);
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (!port) return;

    const poll = async () => {
      try {
        const resp = await fetch(`http://127.0.0.1:${port}/health`);
        if (resp.ok) {
          const data: HealthResponse = await resp.json();
          setHealth(data);
          setConnected(true);
        } else {
          setConnected(false);
        }
      } catch {
        setConnected(false);
      }
    };

    poll();
    intervalRef.current = setInterval(poll, 2000);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [port]);

  return { health, connected, port, connect, disconnect };
}