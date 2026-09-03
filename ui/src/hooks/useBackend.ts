import { useState, useEffect, useRef } from "react";
import type { HealthResponse } from "../lib/types";

interface BackendState {
  health: HealthResponse | null;
  connected: boolean;
  port: number;
}

const CANDIDATE_PORTS = [8080, 8081, 8082, 8083, 8084, 8085];
const POLL_INTERVAL_MS = 1500;

export function useBackend(): BackendState {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [connected, setConnected] = useState(false);
  const [port, setPort] = useState<number>(CANDIDATE_PORTS[0]);
  const mountedRef = useRef(true);
  const discoveredPortRef = useRef<number | null>(null);

  useEffect(() => {
    mountedRef.current = true;

    const poll = async () => {
      // If we already discovered the port, only poll that one
      const portsToTry = discoveredPortRef.current
        ? [discoveredPortRef.current]
        : CANDIDATE_PORTS;

      for (const p of portsToTry) {
        if (!mountedRef.current) return;
        try {
          const resp = await fetch(`http://127.0.0.1:${p}/health`, {
            signal: AbortSignal.timeout(2000),
          });
          if (!mountedRef.current) return;
          if (resp.ok) {
            const data: HealthResponse = await resp.json();
            if (!mountedRef.current) return;
            discoveredPortRef.current = p;
            setPort(p);
            setHealth(data);
            setConnected(true);
            return; // found it, stop trying other ports
          }
        } catch {
          // this port is not responding, try next
        }
      }
      if (!mountedRef.current) return;
      setConnected(false);
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

  return { health, connected, port };
}