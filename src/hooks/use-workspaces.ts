import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL || "";

function getToken(): string {
  // S1-06: أولوية لـ sessionStorage (الأحدث) ثم localStorage (للتوافق مع الجلسات القديمة)
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

// ─── Types ────────────────────────────────────────────────────────────────────

export interface WorkspaceMeta {
  id: string;
  name: string;
  description: string;
  created_at: string;
  file_count: number;
  size_bytes: number;
}

export interface TreeNode {
  type: "file" | "dir";
  name: string;
  path: string;
  size?: number;
  children?: TreeNode[];
}

export interface WorkspaceTree {
  name: string;
  tree: TreeNode[];
}

export type AgentChatEvent =
  | { type: "thinking"; content: string }
  | { type: "tool_use"; tool: string; input: Record<string, unknown>; tool_call_id: string }
  | { type: "tool_result"; tool_call_id: string; result: unknown }
  | { type: "memory_hit"; count: number; preview: string }
  | { type: "text"; content: string }
  | { type: "error"; message: string }
  | { type: "progress"; step: number; total: number; label: string }
  | { type: "file_written"; path: string; lines: number; action: string }
  | { type: "done"; usage: unknown };

// ─── Query Keys ───────────────────────────────────────────────────────────────

const WS_KEYS = {
  all:       ()       => ["workspaces"] as const,
  list:      ()       => ["workspaces", "list"] as const,
  single:    (id: string) => ["workspaces", id] as const,
  tree:      (id: string) => ["workspaces", id, "tree"] as const,
  file:      (id: string, path: string) => ["workspaces", id, "file", path] as const,
};

// ─── Hooks ────────────────────────────────────────────────────────────────────

export function useWorkspaces() {
  return useQuery<WorkspaceMeta[]>({
    queryKey: WS_KEYS.list(),
    queryFn:  () => apiFetch("/workspaces").then(d => d.workspaces ?? d ?? []),
    staleTime: 1000 * 30,
  });
}

export function useWorkspace(id: string) {
  return useQuery<WorkspaceMeta>({
    queryKey: WS_KEYS.single(id),
    queryFn:  () => apiFetch(`/workspaces/${id}`),
    enabled:  !!id,
  });
}

export function useWorkspaceTree(id: string) {
  return useQuery<WorkspaceTree>({
    queryKey: WS_KEYS.tree(id),
    queryFn:  () => apiFetch(`/workspaces/${id}/tree`),
    enabled:  !!id,
    staleTime: 1000 * 15,
  });
}

export function useWorkspaceFile(wsId: string, filePath: string, enabled = false) {
  return useQuery<{ content: string; path: string }>({
    queryKey: WS_KEYS.file(wsId, filePath),
    queryFn:  () => apiFetch(`/workspaces/${wsId}/files/${filePath}`),
    enabled:  enabled && !!wsId && !!filePath,
    staleTime: 1000 * 60,
  });
}

export function useWorkspaceMutations() {
  const qc = useQueryClient();

  const createMut = useMutation({
    mutationFn: (data: { name: string; description?: string }) =>
      apiFetch("/workspaces", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: () => qc.invalidateQueries({ queryKey: WS_KEYS.list() }),
  });

  const updateMut = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { name?: string; description?: string } }) =>
      apiFetch(`/workspaces/${id}`, { method: "PATCH", body: JSON.stringify(data) }),
    onSuccess: (_d, { id }) => {
      qc.invalidateQueries({ queryKey: WS_KEYS.list() });
      qc.invalidateQueries({ queryKey: WS_KEYS.single(id) });
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => apiFetch(`/workspaces/${id}`, { method: "DELETE" }),
    onSuccess: () => qc.invalidateQueries({ queryKey: WS_KEYS.list() }),
  });

  const cloneMut = useMutation({
    mutationFn: ({ id, url }: { id: string; url: string }) =>
      apiFetch(`/workspaces/${id}/clone`, { method: "POST", body: JSON.stringify({ url }) }),
  });

  const writeFileMut = useMutation({
    mutationFn: ({ wsId, path, content }: { wsId: string; path: string; content: string }) =>
      apiFetch(`/workspaces/${wsId}/files/${path}`, {
        method: "PUT",
        body: JSON.stringify({ content }),
      }),
    onSuccess: (_d, { wsId }) => {
      qc.invalidateQueries({ queryKey: WS_KEYS.tree(wsId) });
    },
  });

  const deleteFileMut = useMutation({
    mutationFn: ({ wsId, path }: { wsId: string; path: string }) =>
      apiFetch(`/workspaces/${wsId}/files/${path}`, { method: "DELETE" }),
    onSuccess: (_d, { wsId }) => {
      qc.invalidateQueries({ queryKey: WS_KEYS.tree(wsId) });
    },
  });

  const mkdirMut = useMutation({
    mutationFn: ({ wsId, path }: { wsId: string; path: string }) =>
      apiFetch(`/workspaces/${wsId}/mkdir/${path}`, { method: "POST" }),
    onSuccess: (_d, { wsId }) => {
      qc.invalidateQueries({ queryKey: WS_KEYS.tree(wsId) });
    },
  });

  return {
    create:     createMut,
    update:     updateMut,
    remove:     deleteMut,
    clone:      cloneMut,
    writeFile:  writeFileMut,
    deleteFile: deleteFileMut,
    mkdir:      mkdirMut,
  };
}

// ─── Agent Chat (SSE streaming) ───────────────────────────────────────────────

export async function* streamAgentChat(
  message:     string,
  workspaceId: string,
  sessionId:   string,
  mode         = "coder",
  effort       = "medium",
  history:     Array<{role: string; content: string}> = [],
  signal?:     AbortSignal,
  images?:     Array<{url: string; base64?: string; media_type?: string}>,
): AsyncGenerator<AgentChatEvent> {
  const res = await fetch(`${API_BASE}/api/agent/chat`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${getToken()}`,
    },
    body: JSON.stringify({
      message, workspace_id: workspaceId, session_id: sessionId,
      mode, effort, history,
      ...(images && images.length > 0 ? { images } : {}),
    }),
    signal,
  });

  if (!res.ok || !res.body) {
    yield { type: "error", message: `Server error: ${res.status}` };
    return;
  }

  const reader  = res.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";

  // Dedup: track last seen event fingerprint to prevent duplicate emissions
  let lastTextContent = "";
  let lastEventKey = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });

    const lines = buf.split("\n");
    buf = lines.pop() ?? "";

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed.startsWith("data: ") || trimmed === "data: [DONE]") continue;

      const raw = trimmed.slice(6).trim();
      if (!raw) continue;

      try {
        const event: AgentChatEvent = JSON.parse(raw);

        // ── Deduplication: skip events identical to previous ───────────────
        const eventKey = `${event.type}:${
          event.type === "text" ? (event as {type:"text";content:string}).content.slice(0, 40) :
          event.type === "thinking" ? (event as {type:"thinking";content:string}).content.slice(0, 30) :
          event.type === "tool_use" ? (event as {type:"tool_use";tool:string;input:Record<string,unknown>;tool_call_id:string}).tool :
          JSON.stringify(event).slice(0, 40)
        }`;
        if (eventKey === lastEventKey) continue;
        lastEventKey = eventKey;

        // ── Text dedup: skip if same text content already emitted ──────────
        if (event.type === "text") {
          const textEvent = event as { type: "text"; content: string };
          if (textEvent.content === lastTextContent) continue;
          lastTextContent = textEvent.content;
        }

        // ── Sanitize thinking content — never show JSON or model names ─────
        if (event.type === "thinking") {
          const thinkEvent = event as { type: "thinking"; content: string };
          let cleanContent = thinkEvent.content
            .replace(/\{[^}]{0,200}\}/g, "")      // remove JSON blobs
            .replace(/deepseek[^\s,]*/gi, "")       // hide model names
            .replace(/mimo[^\s,]*/gi, "")
            .replace(/north-mini[^\s,]*/gi, "")
            .replace(/big-pickle[^\s,]*/gi, "")
            .replace(/gpt-[^\s,]*/gi, "")
            .replace(/claude[^\s,]*/gi, "")
            .replace(/\s{2,}/g, " ")
            .trim();
          if (!cleanContent || cleanContent.length < 3) continue; // skip empty thoughts
          yield { ...thinkEvent, content: cleanContent };
          continue;
        }

        yield event;
      } catch {
        // Malformed SSE line — skip silently (never show raw JSON to user)
      }
    }
  }
}
