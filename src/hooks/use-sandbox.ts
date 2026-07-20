import { useMutation } from "@tanstack/react-query";

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

export type SandboxLanguage = "python" | "javascript" | "typescript" | "bash";

export interface SandboxRequest {
  code: string;
  language?: SandboxLanguage;
  timeout_secs?: number;
  env?: Record<string, string>;
}

export interface SandboxResult {
  success: boolean;
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
  language: string;
  compilation_error: string | null;
  timed_out: boolean;
}

export function useExecCode() {
  return useMutation<SandboxResult, Error, SandboxRequest>({
    mutationFn: (data) =>
      apiFetch("/sandbox/exec", { method: "POST", body: JSON.stringify(data) }),
  });
}

export const LANGUAGE_EXTENSIONS: Record<SandboxLanguage, string> = {
  python: ".py",
  javascript: ".js",
  typescript: ".ts",
  bash: ".sh",
};

export const LANGUAGE_COMMENTS: Record<SandboxLanguage, string> = {
  python: "# Python",
  javascript: "// JavaScript",
  typescript: "// TypeScript",
  bash: "# Bash",
};
