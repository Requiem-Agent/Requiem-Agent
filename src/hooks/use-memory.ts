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

const RAG_KEYS = {
  all: () => ["rag"] as const,
  stats: () => ["rag", "stats"] as const,
  memories: (type?: string) => ["rag", "memories", type] as const,
};

export function useRagStats() {
  return useQuery({
    queryKey: RAG_KEYS.stats(),
    queryFn: () => apiFetch("/rag/stats"),
    staleTime: 1000 * 30,
  });
}

export function useMemories(type?: MemoryType) {
  return useQuery({
    queryKey: RAG_KEYS.memories(type),
    queryFn: () => apiFetch(`/rag/memories${type ? `?type=${type}` : ""}`),
    staleTime: 1000 * 30,
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
      qc.invalidateQueries({ queryKey: RAG_KEYS.all() });
    },
  });
}

export function useClearMemory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => apiFetch("/rag/clear", { method: "POST" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: RAG_KEYS.all() });
    },
  });
}

export function useDeleteMemory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiFetch(`/rag/memory/${id}`, { method: "DELETE" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: RAG_KEYS.all() });
    },
  });
}

export async function fetchRagContext(
  query: string,
  sessionId: string,
  maxTokens = 1500,
) {
  try {
    return await apiFetch("/rag/inject-context", {
      method: "POST",
      body: JSON.stringify({ query, session_id: sessionId, max_context_tokens: maxTokens }),
    });
  } catch {
    return { systemContext: "", memoriesUsed: 0, tokenCount: 0, sources: [] };
  }
}

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
    // Fire and forget
  }
}
