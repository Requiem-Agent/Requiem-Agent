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

  // ── Non-SSE response (parallel execution or fallback) ──────────
  // Backend returned a buffered JSON response instead of SSE stream
  if (!contentType.includes("text/event-stream")) {
    const rawText = await r.text();
    if (!rawText.trim()) return;

    // Try to extract clean text from JSON envelope
    const extracted = extractTextFromJson(rawText);
    if (extracted) {
      // Yield in chunks to keep typewriter effect smooth
      const chunkSize = 8;
      for (let i = 0; i < extracted.length; i += chunkSize) {
        yield extracted.slice(i, i + chunkSize);
        // Allow React to re-render between chunks
        await new Promise(res => setTimeout(res, 0));
      }
    } else {
      // Plain text response — yield directly
      yield rawText;
    }
    return;
  }

  // ── SSE streaming path ─────────────────────────────────────────
  const reader = r.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });

    const lines = buf.split("\n");
    // FIXED: keep the last (potentially incomplete) line in the buffer
    buf = lines.pop() ?? "";

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed.startsWith("data: ") || trimmed === "data: [DONE]") continue;
      try {
        const d = JSON.parse(trimmed.slice(6));
        const chunk = d.choices?.[0]?.delta?.content;
        if (chunk) yield chunk;
      } catch {
        // Not valid SSE JSON — try to extract content from a plain JSON object
        // (handles cases where upstream returns non-SSE JSON chunks)
        const extracted = extractTextFromJson(trimmed);
        if (extracted) yield extracted;
      }
    }
  }

  // Process any remaining data in the buffer after stream ends
  const trimmed = buf.trim();
  if (trimmed && trimmed !== "data: [DONE]") {
    if (trimmed.startsWith("data: ")) {
      try {
        const d = JSON.parse(trimmed.slice(6));
        const chunk = d.choices?.[0]?.delta?.content;
        if (chunk) yield chunk;
      } catch {
        const extracted = extractTextFromJson(trimmed.slice(6));
        if (extracted) yield extracted;
      }
    } else {
      // Try extracting from raw JSON
      const extracted = extractTextFromJson(trimmed);
      if (extracted) yield extracted;
    }
  }
}
