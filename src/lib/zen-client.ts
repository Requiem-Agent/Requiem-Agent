/**
 * 🚀 Zen Client — عبر السيرفر المحلي + 10 IPs متدورة
 *
 * الطلبات تروح لنفس السيرفر (لا CORS) والسيرفر يوزعها
 * على Webshare IPs مختلفة → Zen API يشوف IP مختلف كل مرة
 *
 * FIX v2 — إصلاحات:
 * 1. خلل buffer الـ SSE: buf = lines.at(-1) || "" بدلاً من buf = ""
 * 2. معالجة ردود JSON غير المتدفقة (parallel path)
 * 3. استخراج النص من مختلف تنسيقات JSON
 */

const API_BASE = import.meta.env.VITE_API_URL || "";
const API = `${API_BASE}/api/zen/chat`;

// ─── API Key اختياري ────────────────────────────────────────────
export function setApiKey(k: string) { try { localStorage.setItem("zen_key", k); } catch {} }
export function getApiKey(): string | null { try { return localStorage.getItem("zen_key"); } catch { return null; } }
export function hasApiKey(): boolean { return getApiKey() !== null; }

// ─── استخراج النص من أي تنسيق JSON ────────────────────────────
export function extractTextFromJson(raw: string): string | null {
  if (!raw) return null;
  const trimmed = raw.trim();

  // If it doesn't look like JSON at all, return null immediately
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[") && !trimmed.startsWith('"')) {
    // Handle markdown-wrapped JSON (```json\n{...}\n```)
    const mdMatch = trimmed.match(/^```(?:json)?\s*([\s\S]*?)```$/m);
    if (mdMatch) return extractTextFromJson(mdMatch[1]);
    return null;
  }

  // Handle bare JSON string (e.g. "hello world")
  if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
    try { return JSON.parse(trimmed) as string; } catch {}
  }

  try {
    const json = JSON.parse(trimmed);
    // OpenAI streaming delta
    if (json.choices?.[0]?.delta?.content) return json.choices[0].delta.content;
    // OpenAI non-streaming message
    if (json.choices?.[0]?.message?.content) return json.choices[0].message.content;
    // OpenAI simple text
    if (json.choices?.[0]?.text) return json.choices[0].text;
    // Custom {"response": "..."} format (identity shield/agent)
    if (typeof json.response === "string") return json.response;
    // Custom {"text": "..."} format
    if (typeof json.text === "string") return json.text;
    // Custom {"content": "..."} format
    if (typeof json.content === "string") return json.content;
    // Custom {"message": "..."} format — but NOT if it's an error message
    if (typeof json.message === "string" && !json.error && !json.status) return json.message;
    // Nested {"data": {"response": "..."}}
    if (typeof json.data?.response === "string") return json.data.response;
    if (typeof json.data?.content === "string") return json.data.content;
    // Array with single string element
    if (Array.isArray(json) && json.length === 1 && typeof json[0] === "string") return json[0];
    return null;
  } catch {
    // Partial JSON — try to extract content/text/response field with regex
    for (const key of ["content", "text", "response"]) {
      const rx = new RegExp(`"${key}"\\s*:\\s*"((?:[^"\\\\]|\\\\.)*)"`)
      const m = trimmed.match(rx);
      if (m) {
        try { return JSON.parse(`"${m[1]}"`); } catch {
          return m[1].replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\"/g, '"').replace(/\\\\/g, "\\");
        }
      }
    }
    return null;
  }
}

// ─── تنظيف نهائي لأي نص قبل العرض ──────────────────────────────
// يُزيل أي JSON wrapper متبقٍّ ويُعيد نصاً صافياً دائماً
export function cleanDisplayText(raw: string): string {
  if (!raw) return "";
  let text = raw;

  // محاولة استخراج JSON wrapper
  const extracted = extractTextFromJson(raw);
  if (extracted) text = extracted;

  // إصلاح escape sequences
  text = text.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "");

  // إزالة outer quotes إذا بقيت
  if (text.startsWith('"') && text.endsWith('"')) {
    try { text = JSON.parse(text); } catch {}
  }

  return text;
}

// ─── إحصائيات ──────────────────────────────────────────────────
export interface UsageStats {
  used: number;
  limit: number;
  remaining: number;
  hasApiKey: boolean;
  usagePercent: number;
}
export async function fetchUsageStats(): Promise<UsageStats> {
  const token = sessionStorage.getItem("rq_tok") || localStorage.getItem("requiem_token") || "";
  const r = await fetch(`${API_BASE}/api/usage`, {
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

  const token = sessionStorage.getItem("rq_tok") || localStorage.getItem("requiem_token") || "";
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
    const errJson = await r.json().catch(() => ({})) as Record<string, unknown>;
    if (r.status === 429) throw new Error((errJson.error as string) || "Quota exhausted. Try again later.");
    if (r.status === 401) throw new Error("Authentication failed. Please restart the bot.");
    throw new Error(`Server error: ${r.status}`);
  }

  if (!r.body) throw new Error("No response body");

  const contentType = r.headers.get("content-type") || "";

  // ── Non-SSE response (buffered fallback path) ──────────────────
  if (!contentType.includes("text/event-stream")) {
    const rawText = await r.text();
    if (!rawText.trim()) return;

    const extracted = extractTextFromJson(rawText);
    const finalText = extracted ?? rawText;

    // Apply cleanDisplayText one more time to remove any residual JSON wrappers
    const safeText = cleanDisplayText(finalText);
    // Never show raw JSON
    if (!safeText || safeText.trim().startsWith("{") || safeText.trim().startsWith("[")) {
      return; // discard JSON artifacts
    }

    // Stream in small chunks for typewriter effect
    const chunkSize = 6;
    for (let i = 0; i < safeText.length; i += chunkSize) {
      yield safeText.slice(i, i + chunkSize);
      await new Promise(res => setTimeout(res, 8)); // ~120 chars/sec
    }
    return;
  }

  // ── SSE streaming path ─────────────────────────────────────────────
  const reader = r.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";
  let full = "";   // accumulate full response for dedup

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
        const d = JSON.parse(raw);
        // Standard OpenAI streaming delta
        const chunk = d.choices?.[0]?.delta?.content;
        if (typeof chunk === "string" && chunk.length > 0) {
          full += chunk;
          yield chunk;
        }
        // Non-streaming full response embedded in stream (fallback path)
        else if (d.choices?.[0]?.message?.content && full.length === 0) {
          const msg = d.choices[0].message.content as string;
          yield msg;
          full = msg;
        }
      } catch {
        // Not valid JSON — this line was not a valid SSE event, discard it.
        // Never show raw non-JSON text that slipped through the upstream filter.
        // (The backend zen.rs now only emits valid SSE lines, so this path
        //  should rarely trigger.)
      }
    }
  }

  // Process remaining buffer
  const remaining = buf.trim();
  if (remaining && remaining !== "data: [DONE]" && remaining.startsWith("data: ")) {
    const raw = remaining.slice(6).trim();
    try {
      const d = JSON.parse(raw);
      const chunk = d.choices?.[0]?.delta?.content;
      if (typeof chunk === "string" && chunk.length > 0) yield chunk;
    } catch { /* discard malformed trailing data */ }
  }
}