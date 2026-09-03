import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import type { SessionInfo, ProjectInfo, ProviderAccountInfo, HealthResponse } from "../lib/types";

const BASE = "http://127.0.0.1";

function apiUrl(port: number, path: string) {
  return `${BASE}:${port}${path}`;
}

// ── Health ──────────────────────────────────────────────────────────────────

export function useHealth(port: number) {
  return useQuery({
    queryKey: ["health", port],
    queryFn: async () => {
      const resp = await fetch(apiUrl(port, "/health"), {
        signal: AbortSignal.timeout(3000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json() as Promise<HealthResponse>;
    },
    refetchInterval: 5000,
    retry: 1,
  });
}

// ── Sessions ────────────────────────────────────────────────────────────────

export function useSessions(port: number) {
  return useQuery({
    queryKey: ["sessions", port],
    queryFn: async () => {
      const resp = await fetch(apiUrl(port, "/api/sessions"), {
        signal: AbortSignal.timeout(3000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json() as Promise<SessionInfo[]>;
    },
    refetchInterval: 3000,
  });
}

export function useSpawnSession(port: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (params: { agentBinary: string; accountId?: string; workspacePath?: string }) => {
      const body: Record<string, string> = { agent_binary: params.agentBinary };
      if (params.accountId) body.account_id = params.accountId;
      if (params.workspacePath) body.workspace_path = params.workspacePath;
      const resp = await fetch(apiUrl(port, "/api/sessions"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(10000),
      });
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({ error: `HTTP ${resp.status}` }));
        throw new Error(err.error || `HTTP ${resp.status}`);
      }
      return resp.json() as Promise<{ session_id: string; status: string; agent_binary: string; workspace_path: string }>;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["sessions", port] });
    },
  });
}

export function useDeleteSession(port: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (sessionId: string) => {
      const resp = await fetch(apiUrl(port, `/api/sessions/${sessionId}`), {
        method: "DELETE",
        signal: AbortSignal.timeout(5000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json();
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["sessions", port] });
    },
  });
}

// ── Projects ────────────────────────────────────────────────────────────────

export function useProjects(port: number) {
  return useQuery({
    queryKey: ["projects", port],
    queryFn: async () => {
      const resp = await fetch(apiUrl(port, "/api/projects"), {
        signal: AbortSignal.timeout(3000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json() as Promise<ProjectInfo[]>;
    },
  });
}

export function useCreateProject(port: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (params: { name: string; path: string }) => {
      const resp = await fetch(apiUrl(port, "/api/projects"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(params),
        signal: AbortSignal.timeout(5000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json() as Promise<ProjectInfo>;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["projects", port] });
    },
  });
}

// ── Accounts ────────────────────────────────────────────────────────────────

export function useAccounts(port: number) {
  return useQuery({
    queryKey: ["accounts", port],
    queryFn: async () => {
      const resp = await fetch(apiUrl(port, "/api/accounts"), {
        signal: AbortSignal.timeout(3000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json() as Promise<ProviderAccountInfo[]>;
    },
  });
}

export function useCreateAccount(port: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (params: { provider: string; label: string }) => {
      const resp = await fetch(apiUrl(port, "/api/accounts"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(params),
        signal: AbortSignal.timeout(5000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json();
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["accounts", port] });
    },
  });
}

// ── Agents ──────────────────────────────────────────────────────────────────

export function useAgents(port: number) {
  return useQuery({
    queryKey: ["agents", port],
    queryFn: async () => {
      const resp = await fetch(apiUrl(port, "/api/agents"), {
        signal: AbortSignal.timeout(3000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json() as Promise<Array<{ name: string; binary: string; status: string; path: string | null; version: string | null }>>;
    },
    staleTime: 30000,
  });
}

// ── Orchestrate ─────────────────────────────────────────────────────────────

export function useOrchestrate(port: number) {
  return useMutation({
    mutationFn: async (objective: string) => {
      const resp = await fetch(apiUrl(port, "/api/orchestrate"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ objective }),
        signal: AbortSignal.timeout(15000),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      return resp.json();
    },
  });
}