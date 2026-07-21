// use-preferences.ts — S6-03: React hook for GET/PUT /api/preferences
//
// يُوفّر:
//   usePreferences()      → جلب تفضيلات المستخدم
//   useUpdatePreferences() → تحديث تفضيلات المستخدم (optimistic update)
//   useApiKeys()          → جلب مفاتيح API المحفوظة
//   useSaveApiKey()       → حفظ مفتاح API جديد
//   useDeleteApiKey()     → حذف مفتاح API

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface UserPreferences {
  // UI / UX
  theme: "dark" | "light" | "system";
  language: string;
  compact_mode: boolean;
  show_timestamps: boolean;
  enable_animations: boolean;
  // Agent behaviour
  default_model: string;
  default_mode: "chat" | "orchestrator" | "code";
  max_tokens: number;
  temperature: number;
  system_prompt?: string;
  stream_responses: boolean;
  show_thinking: boolean;
  // Notifications
  notify_on_complete: boolean;
  notify_on_error: boolean;
  notify_on_mention: boolean;
  // Privacy
  save_history: boolean;
  share_analytics: boolean;
}

export type UpdatePreferencesPayload = Partial<UserPreferences>;

export interface ApiKeyRecord {
  id: string;
  provider: string;
  key_hint: string;
  created_at: string;
  updated_at: string;
}

export interface SaveApiKeyPayload {
  provider: string;
  api_key: string;
}

// ─── API helpers ──────────────────────────────────────────────────────────────

function getAuthHeader(): Record<string, string> {
  const token = localStorage.getItem("requiem_token") ?? sessionStorage.getItem("requiem_token");
  return token ? { Authorization: `Bearer ${token}` } : {};
}

const API_BASE = import.meta.env.VITE_API_URL ?? "/api";

async function fetchPreferences(): Promise<UserPreferences> {
  const res = await fetch(`${API_BASE}/preferences`, {
    headers: { ...getAuthHeader(), "Content-Type": "application/json" },
  });
  if (!res.ok) throw new Error(`Failed to fetch preferences: ${res.status}`);
  const json = await res.json();
  return json.data as UserPreferences;
}

async function putPreferences(payload: UpdatePreferencesPayload): Promise<{ updated_fields: string[] }> {
  const res = await fetch(`${API_BASE}/preferences`, {
    method: "PUT",
    headers: { ...getAuthHeader(), "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error((err as { error?: string }).error ?? `Failed to update preferences: ${res.status}`);
  }
  const json = await res.json();
  return json.data;
}

async function fetchApiKeys(): Promise<ApiKeyRecord[]> {
  const res = await fetch(`${API_BASE}/user-api-keys`, {
    headers: { ...getAuthHeader() },
  });
  if (!res.ok) throw new Error(`Failed to fetch API keys: ${res.status}`);
  const json = await res.json();
  return json.data as ApiKeyRecord[];
}

async function postApiKey(payload: SaveApiKeyPayload): Promise<ApiKeyRecord> {
  const res = await fetch(`${API_BASE}/user-api-keys`, {
    method: "POST",
    headers: { ...getAuthHeader(), "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new Error((err as { error?: string }).error ?? `Failed to save API key: ${res.status}`);
  }
  const json = await res.json();
  return json.data as ApiKeyRecord;
}

async function deleteApiKeyById(keyId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/user-api-keys/${keyId}`, {
    method: "DELETE",
    headers: { ...getAuthHeader() },
  });
  if (!res.ok) throw new Error(`Failed to delete API key: ${res.status}`);
}

// ─── Hooks ────────────────────────────────────────────────────────────────────

/** جلب تفضيلات المستخدم من الـ backend */
export function usePreferences() {
  return useQuery<UserPreferences, Error>({
    queryKey: ["preferences"],
    queryFn: fetchPreferences,
    staleTime: 5 * 60 * 1000, // 5 دقائق
    retry: 2,
  });
}

/** تحديث تفضيلات المستخدم مع optimistic update */
export function useUpdatePreferences() {
  const qc = useQueryClient();

  return useMutation<{ updated_fields: string[] }, Error, UpdatePreferencesPayload>({
    mutationFn: putPreferences,
    // Optimistic update — نُحدّث الـ cache فوراً قبل انتهاء الطلب
    onMutate: async (newPrefs) => {
      await qc.cancelQueries({ queryKey: ["preferences"] });
      const previous = qc.getQueryData<UserPreferences>(["preferences"]);
      qc.setQueryData<UserPreferences>(["preferences"], (old) =>
        old ? { ...old, ...newPrefs } : (newPrefs as UserPreferences)
      );
      return { previous };
    },
    // في حالة الخطأ — نُعيد القيمة القديمة
    onError: (_err, _vars, context) => {
      const ctx = context as { previous?: UserPreferences } | undefined;
      if (ctx?.previous) {
        qc.setQueryData(["preferences"], ctx.previous);
      }
    },
    // بعد النجاح أو الفشل — نُعيد جلب البيانات الحقيقية
    onSettled: () => {
      qc.invalidateQueries({ queryKey: ["preferences"] });
    },
  });
}

/** جلب مفاتيح API المحفوظة */
export function useApiKeys() {
  return useQuery<ApiKeyRecord[], Error>({
    queryKey: ["api-keys"],
    queryFn: fetchApiKeys,
    staleTime: 2 * 60 * 1000,
    retry: 2,
  });
}

/** حفظ مفتاح API جديد */
export function useSaveApiKey() {
  const qc = useQueryClient();
  return useMutation<ApiKeyRecord, Error, SaveApiKeyPayload>({
    mutationFn: postApiKey,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["api-keys"] });
    },
  });
}

/** حذف مفتاح API */
export function useDeleteApiKey() {
  const qc = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: deleteApiKeyById,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["api-keys"] });
    },
  });
}
