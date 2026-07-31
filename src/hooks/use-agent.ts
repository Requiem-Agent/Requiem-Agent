import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL || "";

function getToken(): string {
  return sessionStorage.getItem("rq_tok") || localStorage.getItem("requiem_token") || "";
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

// GET /api/agent/status (was: /agent/protocol/state)
export function useAgentState() {
  return useQuery({
    queryKey: AGENT_KEYS.state(),
    queryFn: () => apiFetch("/agent/status"),
    refetchInterval: 5000,
  });
}

// POST /api/agent/mode/set (was: /agent/protocol/think — no direct equivalent)
export function useAgentThink() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { task: string; mode?: "fast" | "standard" | "deep"; context?: string }) =>
      apiFetch("/agent/mode/set", { method: "POST", body: JSON.stringify({ mode: data.mode ?? "autonomous", reason: data.task }) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AGENT_KEYS.state() });
    },
  });
}

// POST /api/agent/mode/set (was: /agent/protocol/mode)
export function useSetAgentMode() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { mode: string; reason?: string }) =>
      apiFetch("/agent/mode/set", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AGENT_KEYS.state() });
    },
  });
}

// GET /api/user/question/pending (was: /agent/questions/pending)
export function usePendingQuestions() {
  return useQuery({
    queryKey: AGENT_KEYS.questions(),
    queryFn: () => apiFetch("/user/question/pending"),
    refetchInterval: 10000,
  });
}

// PUT /api/user/question/:id/answer (was: POST /agent/questions/answer)
export function useAnswerQuestion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { question_id: string; answer: string }) =>
      apiFetch(`/user/question/${data.question_id}/answer`, {
        method: "PUT",
        body: JSON.stringify({ answer: data.answer }),
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: AGENT_KEYS.questions() });
    },
  });
}

// GET /api/agent/synergy/history (unchanged — exists in backend)
export function useSynergyHistory() {
  return useQuery({
    queryKey: AGENT_KEYS.synergyHistory(),
    queryFn: () => apiFetch("/agent/synergy/history"),
    staleTime: 1000 * 60,
  });
}

// GET /api/identity/stats (unchanged — exists in backend)
export function useIdentityStats() {
  return useQuery({
    queryKey: AGENT_KEYS.identityStats(),
    queryFn: () => apiFetch("/identity/stats"),
    staleTime: 1000 * 60 * 5,
  });
}

// POST /api/enforce/check-security (was: /enforce/scan)
export function useScanCode() {
  return useMutation({
    mutationFn: (data: { code?: string; content?: string }) =>
      apiFetch("/enforce/check-security", { method: "POST", body: JSON.stringify({ content: data.code ?? data.content ?? "" }) }),
  });
}

// POST /api/formats/:name/convert (was: /formats/convert with {from,to,content})
// Use `to` as the target format name
export function useConvertFormat() {
  return useMutation({
    mutationFn: (data: { from: string; to: string; content: string }) =>
      apiFetch(`/formats/${encodeURIComponent(data.to)}/convert`, {
        method: "POST",
        body: JSON.stringify({ from: data.from, content: data.content }),
      }),
  });
}
