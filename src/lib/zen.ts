/**
 * عميل OpenCode Zen — الطلبات ترسل من WebView المستخدم مباشرة
 * 
 * #### آلية الكوتا
 * - **مستخدم مجهول**: `Authorization: Bearer public` → ipRateLimiter (كوتا IP)
 * - **مستخدم API Key**: `Authorization: Bearer <KEY>` → keyRateLimiter (كوتا الحساب)
 * 
 * #### لماذا من المتصفح؟
 * كل طلب يُرسل من IP المستخدم → كل مستخدم يستهلك من كوتا IP الخاصة به
 * وليس من IP مساحة Hugging Face (التي يشترك فيها الجميع)
 */

// ─── إدارة الجلسة ──────────────────────────────────────────────────

const SESSION_KEY = "requiem_zen_session";
const API_KEY_STORAGE_KEY = "requiem_zen_api_key";

/**
 * الحصول على Session ID فريد لكل مستخدم.
 * يُخزن في localStorage (أو Telegram CloudStorage في WebView)
 */
function getSessionId(): string {
  try {
    let sid = localStorage.getItem(SESSION_KEY);
    if (!sid) {
      sid = `ses_${crypto.randomUUID()}`;
      localStorage.setItem(SESSION_KEY, sid);
    }
    return sid;
  } catch {
    // localStorage غير متاح (بعض WebViews)
    return `ses_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  }
}

/**
 * الحصول على API Key المخزن للمستخدم (إن وجد)
 */
function getApiKey(): string | null {
  try {
    // 1. localStorage
    const stored = localStorage.getItem(API_KEY_STORAGE_KEY);
    if (stored) return stored;

    // 2. Telegram CloudStorage (في وضع WebView)
    if (typeof Telegram !== "undefined" && Telegram?.WebApp?.CloudStorage) {
      // ملاحظة: CloudStorage يستخدم callback, هذا تبسيط
      return null;
    }
  } catch {
    // ignore
  }
  return null;
}

/**
 * حفظ API Key للمستخدم
 */
export function setApiKey(key: string): void {
  try {
    localStorage.setItem(API_KEY_STORAGE_KEY, key);
    // محاولة التخزين في Telegram CloudStorage
    if (typeof Telegram !== "undefined" && Telegram?.WebApp?.CloudStorage) {
      Telegram.WebApp.CloudStorage.setItem(API_KEY_STORAGE_KEY, key);
    }
  } catch {
    // ignore in non-browser environments
  }
}

/**
 * حذف API Key المحفوظ
 */
export function clearApiKey(): void {
  try {
    localStorage.removeItem(API_KEY_STORAGE_KEY);
    if (typeof Telegram !== "undefined" && Telegram?.WebApp?.CloudStorage) {
      Telegram.WebApp.CloudStorage.removeItem(API_KEY_STORAGE_KEY);
    }
  } catch {
    // ignore
  }
}

/**
 * هل المستخدم لديه API Key مخصص؟
 */
export function hasApiKey(): boolean {
  return getApiKey() !== null;
}

// ─── عداد الطلبات المحلي ────────────────────────────────────────────

const COUNTS_KEY = "requiem_zen_counts";
const DAILY_LIMIT_ANONYMOUS = 100; // حد تقديري للمستخدم المجهول

interface DailyCounts {
  date: string; // YYYY-MM-DD
  count: number;
}

function getDailyCounts(): DailyCounts {
  try {
    const raw = localStorage.getItem(COUNTS_KEY);
    if (raw) {
      const data = JSON.parse(raw) as DailyCounts;
      const today = new Date().toISOString().slice(0, 10);
      if (data.date === today) return data;
    }
  } catch {
    // ignore
  }
  return { date: new Date().toISOString().slice(0, 10), count: 0 };
}

function incrementDailyCount(): void {
  try {
    const counts = getDailyCounts();
    counts.count++;
    localStorage.setItem(COUNTS_KEY, JSON.stringify(counts));
  } catch {
    // ignore
  }
}

/**
 * الحصول على إحصائيات الاستخدام للمستخدم الحالي
 */
export function getUsageStats(): { used: number; limit: number; remaining: number } {
  const counts = getDailyCounts();
  const limit = hasApiKey() ? 500 : DAILY_LIMIT_ANONYMOUS;
  return {
    used: counts.count,
    limit,
    remaining: Math.max(0, limit - counts.count),
  };
}

// ─── الوظيفة الأساسية: إرسال طلب دردشة ─────────────────────────────

export async function* streamZenChat(
  model: string,
  messages: { role: string; content: string }[],
  signal?: AbortSignal
) {
  const apiKey = getApiKey() || "public";
  const sessionId = getSessionId();
  const requestId = `msg_${crypto.randomUUID()}`;

  const response = await fetch("https://opencode.ai/zen/v1/chat/completions", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Authorization": `Bearer ${apiKey}`,
      "x-opencode-session": sessionId,
      "x-opencode-request": requestId,
    },
    body: JSON.stringify({ model, messages, stream: true }),
    signal,
  });

  if (!response.ok) {
    if (response.status === 429) {
      throw new Error("Quota exhausted. Please wait and try again.");
    }
    if (response.status === 401) {
      throw new Error("Invalid API key. Check your key or remove it to use free tier.");
    }
    throw new Error(`Chat API error: ${response.status}`);
  }

  if (!response.body) throw new Error("No response body");

  // بعد نجاح الطلب، سجل الاستخدام محلياً
  incrementDailyCount();

  const reader = response.body.getReader();
  const decoder = new TextDecoder("utf-8");
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n");
    buffer = lines.pop() || "";

    for (const line of lines) {
      if (line.startsWith("data: ") && line.trim() !== "data: [DONE]") {
        try {
          const data = JSON.parse(line.slice(6));
          if (data.choices?.[0]?.delta?.content) {
            yield data.choices[0].delta.content;
          }
        } catch (e) {
          // ignore parse errors
        }
      }
    }
  }
}
