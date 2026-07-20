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

// ── Types ───────────────────────────────────────────────────────────────────
export type MemoryType = "code" | "fact" | "preference" | "context" | "error";
export type MemoryPriority = "critical" | "high" | "medium" | "low";

export interface Memory {
  id: string;
  userId: string;
  sessionId: string | null;
  content: string;
  memory_type: MemoryType;
  priority: MemoryPriority;
  access_count: number;
  created_at: string;
  updated_at: string;
  score?: number;
}

export interface RagStats {
  total: number;
  by_type: Record<string, number>;
  by_priority: Record<string, number>;
}

export interface InjectedContext {
  systemContext: string;
  memoriesUsed: number;
  tokenCount: number;
  sources: Array<{ id: string; type: MemoryType; score: number }>;
}

// ── Query Keys ────────────────────────────────────────────────────────────────
export const RAG_KEYS = {
  stats: () => ["rag", "stats"] as const,
  memories: (filter?: Record<string, string>) => ["rag", "memories", filter] as const,
};

// ── Hooks ────────────────────────────────────────────────────────────────────
export function useRagStats() {
  return useQuery<RagStats>({
    queryKey: RAG_KEYS.stats(),
    queryFn: () => apiFetch("/rag/stats"),
    staleTime: 30_000,
    retry: false,
  });
}

export function useMemoryList(opts: {
  limit?: number;
  offset?: number;
  sessionId?: string;
  type?: MemoryType;
} = {}) {
  const params = new URLSearchParams();
  if (opts.limit)     params.set("limit",      String(opts.limit));
  if (opts.offset)    params.set("offset",     String(opts.offset));
  if (opts.sessionId) params.set("session_id", opts.sessionId);
  if (opts.type)      params.set("type",        opts.type);
  const qs = params.toString() ? `?${params.toString()}` : "";

  return useQuery<{ memories: Memory[]; count: number }>({
    queryKey: RAG_KEYS.memories({ ...opts } as Record<string, string>),
    queryFn: () => apiFetch(`/rag/memories${qs}`),
    staleTime: 20_000,
    retry: false,
  });
}

export function useDeleteMemory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      apiFetch(`/rag/memory/${id}`, { method: "DELETE" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: RAG_KEYS.stats() });
      qc.invalidateQueries({ queryKey: ["rag", "memories"] });
    },
  });
}

export function useClearMemory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (sessionId?: string) =>
      apiFetch("/rag/clear", {
        method: "POST",
        body: JSON.stringify(sessionId ? { session_id: sessionId } : {}),
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: RAG_KEYS.stats() });
      qc.invalidateQueries({ queryKey: ["rag", "memories"] });
    },
  });
}

export function useStoreMemory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      content: string;
      memory_type?: MemoryType;
      priority?: MemoryPriority;
      session_id?: string;
    }) => apiFetch("/rag/store", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: RAG_KEYS.stats() });
    },
  });
}

// ── Inject RAG context before a chat message ─────────────────────────────────
export async function fetchRagContext(
  query: string,
  sessionId: string,
  maxTokens = 1500,
): Promise<InjectedContext> {
  try {
    return await apiFetch("/rag/inject-context", {
      method: "POST",
      body: JSON.stringify({ query, session_id: sessionId, max_context_tokens: maxTokens }),
    });
  } catch {
    return { systemContext: "", memoriesUsed: 0, tokenCount: 0, sources: [] };
  }
}

// ── Auto-store memories after a chat turn ────────────────────────────────────
export async function autoStoreMemory(
  userMessage: string,
  assistantResponse: string,
  sessionId: string,
): Promise<void> {
  try {
    await apiFetch("/rag/auto-store", {
      method: "POST",
      body: JSON.stringify({
        user_message: userMessage,
        assistant_response: assistantResponse,
        session_id: sessionId,
      }),
    });
  } catch {
    // Fire and forget — fail silently
  }
}
