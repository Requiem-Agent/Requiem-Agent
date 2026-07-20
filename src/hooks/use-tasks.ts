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

export type TaskStatus = "pending" | "in_progress" | "completed" | "failed" | "blocked";

export interface Task {
  id: string;
  description: string;
  status: TaskStatus;
  parent_id?: string | null;
  assigned_model?: string | null;
  priority?: number;
  dependencies?: string[];
  children?: Task[];
  created_at: string;
  updated_at: string;
}

export interface TaskProgress {
  total: number;
  completed: number;
  failed: number;
  pending: number;
  percent: number;
}

const TASK_KEYS = {
  all: () => ["tasks"] as const,
  tree: (id: string) => ["tasks", id] as const,
  progress: (id: string) => ["tasks", id, "progress"] as const,
};

export function useTaskTree(id: string) {
  return useQuery({
    queryKey: TASK_KEYS.tree(id),
    queryFn: () => apiFetch(`/tasks/${id}`),
    enabled: !!id,
    refetchInterval: (query) => {
      const data = query.state.data as Task | undefined;
      if (!data) return 5000;
      const hasRunning = hasInProgress(data);
      return hasRunning ? 3000 : false;
    },
  });
}

export function useTaskProgress(id: string) {
  return useQuery({
    queryKey: TASK_KEYS.progress(id),
    queryFn: () => apiFetch(`/tasks/${id}/progress`),
    enabled: !!id,
    refetchInterval: 4000,
  });
}

export function useDecomposeTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { description: string; owner?: string }) =>
      apiFetch("/tasks/decompose", { method: "POST", body: JSON.stringify(data) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: TASK_KEYS.all() });
    },
  });
}

export function useUpdateTaskStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: string; status: TaskStatus }) =>
      apiFetch(`/tasks/${id}/status`, {
        method: "PATCH",
        body: JSON.stringify({ status }),
      }),
    onSuccess: (_data, variables) => {
      qc.invalidateQueries({ queryKey: TASK_KEYS.all() });
      qc.invalidateQueries({ queryKey: TASK_KEYS.tree(variables.id) });
    },
  });
}

function hasInProgress(task: Task): boolean {
  if (task.status === "in_progress") return true;
  if (task.children) return task.children.some(hasInProgress);
  return false;
}
