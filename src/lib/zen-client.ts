/**
 * 🚀 Zen Client — عبر السيرفر المحلي + 10 IPs متدورة
 * 
 * الطلبات تروح لنفس السيرفر (لا CORS) والسيرفر يوزعها
 * على 10 Webshare IPs مختلفة → Zen API يشوف IP مختلف كل مرة
 */

const API = "/api/zen/chat";

// ─── API Key اختياري ────────────────────────────────────────────
export function setApiKey(k: string) { try { localStorage.setItem("zen_key", k); } catch {} }
export function getApiKey(): string | null { try { return localStorage.getItem("zen_key"); } catch { return null; } }
export function hasApiKey(): boolean { return getApiKey() !== null; }

// ─── إحصائيات ──────────────────────────────────────────────────
export interface UsageStats {
  used: number;
  limit: number;
  remaining: number;
  hasApiKey: boolean;
  usagePercent: number;
}
export async function fetchUsageStats(): Promise<UsageStats> {
  const token = localStorage.getItem("requiem_token") || "";
  const r = await fetch("/api/usage", {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });
  const d = await r.json();
  return {
    used: d.quotaReadUsed ?? 0,
    limit: d.quotaRead ?? 50000,
    remaining: d.quotaReadRemaining ?? 50000,
    hasApiKey: hasApiKey(),
    usagePercent: Math.round(((d.quotaReadUsed ?? 0) / (d.quotaRead ?? 50000)) * 100),
  };
}

// ─── الدردشة — عبر السيرفر ─────────────────────────────────────
export async function* streamZenChat(
  model: string,
  messages: { role: string; content: string }[],
  signal?: AbortSignal
) {
  const body: Record<string, unknown> = { model, messages, stream: true };
  if (hasApiKey()) body.apiKey = getApiKey();

  const token = localStorage.getItem("requiem_token") || "";
  const r = await fetch(API, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
    signal,
  });

  if (!r.ok) {
    if (r.status === 429) throw new Error((await r.json().catch(() => ({}))).error || "Quota exhausted");
    throw new Error(`Chat error: ${r.status}`);
  }

  if (!r.body) throw new Error("No response body");

  const reader = r.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });
    for (const line of buf.split("\n")) {
      if (line.startsWith("data: ") && line.trim() !== "data: [DONE]") {
        try {
          const d = JSON.parse(line.slice(6));
          if (d.choices?.[0]?.delta?.content) yield d.choices[0].delta.content;
        } catch {}
      }
    }
    buf = "";
  }
}
