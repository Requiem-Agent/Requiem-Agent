import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL || "";

function getToken(): string {
  return localStorage.getItem("requiem_token") || "";
}

async function apiFetch(path: string, opts: RequestInit = {}) {
  const res = await fetch(`${API_BASE}/api${path}`, {
    ...opts,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${getToken()}`,
      ...((opts.headers as Record<string, string>) || {}),
    },
  });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export interface ThinkingStep {
  stage: string;
  reasoning: string;
  confidence: number;
  tokens_used: number;
  duration_ms: number;
  artifacts: unknown[];
  tools_considered: string[];
  selected_tool: string | null;
}

export interface AgentState {
  mode: string;
  thinking: boolean;
  steps: number;
  started: string;
}

const AGENT_KEYS = {
  state: () => ["agent", "state"] as const,
  questions: () => ["agent", "questions"] as const,
  synergyHistory: () => ["agent", "synergy", "history"] as const,
  identityStats: () => ["agent", "identity", "stats"] as const,
};

export function useAgentState() {
  return useQuery({
    queryKey: AGENT_KEYS.state(),
    queryFn: () => apiFetch("/agent/protocol/state"),
    refetchInterval: 5000,
  });
}

export function useAgentThink() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { task: string; mode?: "fast" | "standard" | "deep"; context?: string }) =>
      apiFetch("/agent/protocol/think", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AGENT_KEYS.state() });
    },
  });
}

export function useSetAgentMode() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { mode: string; reason?: string }) =>
      apiFetch("/agent/protocol/mode", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AGENT_KEYS.state() });
    },
  });
}

export function usePendingQuestions() {
  return useQuery({
    queryKey: AGENT_KEYS.questions(),
    queryFn: () => apiFetch("/agent/questions/pending"),
    refetchInterval: 10000,
  });
}

export function useAnswerQuestion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { question_id: string; answer: string }) =>
      apiFetch("/agent/questions/answer", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AGENT_KEYS.questions() });
    },
  });
}

export function useSynergyHistory() {
  return useQuery({
    queryKey: AGENT_KEYS.synergyHistory(),
    queryFn: () => apiFetch("/agent/synergy/history"),
    staleTime: 1000 * 60,
  });
}

export function useIdentityStats() {
  return useQuery({
    queryKey: AGENT_KEYS.identityStats(),
    queryFn: () => apiFetch("/identity/stats"),
    staleTime: 1000 * 60 * 5,
  });
}

export function useScanCode() {
  return useMutation({
    mutationFn: (data: { code?: string; content?: string }) =>
      apiFetch("/enforce/scan", { method: "POST", body: JSON.stringify(data) }),
  });
}

export function useConvertFormat() {
  return useMutation({
    mutationFn: (data: { from: string; to: string; content: string }) =>
      apiFetch("/formats/convert", { method: "POST", body: JSON.stringify(data) }),
  });
}
