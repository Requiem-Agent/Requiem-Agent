import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL || "";

function getToken(): string {
  return localStorage.getItem("requiem_token") || "";
}

async function apiFetch(path: string, opts: RequestInit = {}) {
  const res = await fetch(`${API_BASE}/api${path}`, {
    ...opts,
    headers: {
      Authorization: `Bearer ${getToken()}`,
      ...((opts.headers as Record<string, string>) || {}),
    },
  });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export interface AgentFile {
  name: string;
  size: number;
  created_at: string;
  modified_at?: string;
  content_type?: string;
}

const FILE_KEYS = {
  all: () => ["files"] as const,
  list: () => ["files", "list"] as const,
  content: (name: string) => ["files", "content", name] as const,
};

export function useFiles() {
  return useQuery<AgentFile[]>({
    queryKey: FILE_KEYS.list(),
    queryFn: () => apiFetch("/files"),
    staleTime: 1000 * 30,
    select: (data: any) => data.files ?? data ?? [],
  });
}

export function useFileContent(name: string, enabled = false) {
  return useQuery<{ name: string; content: string }>({
    queryKey: FILE_KEYS.content(name),
    queryFn: () => apiFetch(`/files/${encodeURIComponent(name)}`),
    enabled: enabled && !!name,
    staleTime: 1000 * 60,
  });
}

export function useDeleteFile() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) =>
      apiFetch(`/files/${encodeURIComponent(name)}`, { method: "DELETE" }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: FILE_KEYS.list() });
    },
  });
}

export function useUploadFile() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ name, content }: { name: string; content: string }) => {
      const formData = new FormData();
      const blob = new Blob([content], { type: "text/plain" });
      formData.append("file", blob, name);
      const res = await fetch(`${API_BASE}/api/files/upload`, {
        method: "POST",
        headers: { Authorization: `Bearer ${getToken()}` },
        body: formData,
      });
      if (!res.ok) throw new Error("Upload failed");
      return res.json();
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: FILE_KEYS.list() });
    },
  });
}
