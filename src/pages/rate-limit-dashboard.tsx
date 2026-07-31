// rate-limit-dashboard.tsx — S9-04: Rate Limiting Dashboard
// صفحة تعرض استخدام الـ rate limits الحالي للمستخدم

import { useQuery } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL ?? "http://localhost:7860";

interface RateLimitStatus {
  endpoint: string;
  label: string;
  current: number;
  limit: number;
  window_secs: number;
  reset_at: string;
}

interface RateLimitResponse {
  success: boolean;
  data: {
    user_id: string;
    limits: RateLimitStatus[];
    global_limit_hit: boolean;
  };
}

async function fetchRateLimits(): Promise<RateLimitResponse> {
  const res = await fetch(`${API_BASE}/api/rate-limits/status`, {
    credentials: "include",
  });
  if (!res.ok) throw new Error(`Failed to fetch rate limits: ${res.status}`);
  return res.json();
}

function useRateLimits() {
  return useQuery({
    queryKey: ["rate-limits"],
    queryFn: fetchRateLimits,
    refetchInterval: 10_000, // تحديث كل 10 ثوانٍ
    staleTime: 5_000,
  });
}

function getUsageColor(pct: number): string {
  if (pct >= 90) return "#ef4444";
  if (pct >= 70) return "#f59e0b";
  return "#10b981";
}

function formatResetTime(resetAt: string): string {
  const reset = new Date(resetAt);
  const now = new Date();
  const diffSecs = Math.max(0, Math.floor((reset.getTime() - now.getTime()) / 1000));
  if (diffSecs < 60) return `${diffSecs} ثانية`;
  return `${Math.floor(diffSecs / 60)} دقيقة`;
}

interface LimitCardProps {
  limit: RateLimitStatus;
}

function LimitCard({ limit }: LimitCardProps) {
  const pct = Math.min(100, Math.round((limit.current / limit.limit) * 100));
  const color = getUsageColor(pct);
  const remaining = limit.limit - limit.current;

  return (
    <div className="limit-card">
      <div className="limit-header">
        <span className="limit-label">{limit.label}</span>
        <span className="limit-endpoint">{limit.endpoint}</span>
      </div>

      <div className="limit-numbers">
        <span className="limit-current" style={{ color }}>
          {limit.current}
        </span>
        <span className="limit-separator">/</span>
        <span className="limit-max">{limit.limit}</span>
        <span className="limit-unit">طلب/{Math.floor(limit.window_secs / 60)} دقيقة</span>
      </div>

      <div className="progress-bar-container">
        <div
          className="progress-bar"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>

      <div className="limit-footer">
        <span className="remaining" style={{ color }}>
          {remaining} طلب متبقٍ
        </span>
        <span className="reset-time">
          يُعاد الضبط خلال {formatResetTime(limit.reset_at)}
        </span>
      </div>
    </div>
  );
}

export default function RateLimitDashboard() {
  const { data, isLoading, error, refetch } = useRateLimits();

  return (
    <div className="rate-limit-page" dir="rtl">
      <div className="page-header">
        <div>
          <h1 className="page-title">🚦 حدود الاستخدام</h1>
          <p className="page-subtitle">
            مراقبة استخدامك الحالي لـ API endpoints
          </p>
        </div>
        <button className="refresh-btn" onClick={() => refetch()}>
          🔄 تحديث
        </button>
      </div>

      {isLoading && (
        <div className="loading">
          <div className="spinner" />
          <p>جارٍ التحميل...</p>
        </div>
      )}

      {error && (
        <div className="error-banner">
          ⚠️ فشل تحميل البيانات: {error instanceof Error ? error.message : "خطأ غير معروف"}
        </div>
      )}

      {data && (
        <>
          {data.data.global_limit_hit && (
            <div className="global-limit-warning">
              🚨 <strong>تحذير:</strong> لقد تجاوزت الحد الأقصى العام. بعض الطلبات ستُرفَض.
            </div>
          )}

          <div className="limits-grid">
            {data.data.limits.map((limit) => (
              <LimitCard key={limit.endpoint} limit={limit} />
            ))}
          </div>

          <div className="info-section">
            <h3>ℹ️ معلومات</h3>
            <ul>
              <li>الحدود تُطبَّق على مستوى المستخدم (user ID من JWT)</li>
              <li>تجاوز الحد يُرجع HTTP 429 Too Many Requests</li>
              <li>الحدود تُعاد تلقائياً بعد انتهاء النافذة الزمنية</li>
              <li>للحصول على حدود أعلى، تواصل مع المسؤول</li>
            </ul>
          </div>
        </>
      )}

      <style>{`
        .rate-limit-page {
          max-width: 900px;
          margin: 0 auto;
          padding: 2rem 1rem;
          color: #e2e8f0;
          font-family: 'Segoe UI', Tahoma, sans-serif;
        }
        .page-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 1.5rem;
        }
        .page-title { font-size: 1.75rem; font-weight: 700; margin: 0 0 0.25rem; }
        .page-subtitle { color: #94a3b8; margin: 0; font-size: 0.9rem; }
        .refresh-btn {
          background: #1e293b;
          border: 1px solid #334155;
          color: #94a3b8;
          padding: 0.5rem 1rem;
          border-radius: 8px;
          cursor: pointer;
          font-size: 0.875rem;
          transition: all 0.2s;
        }
        .refresh-btn:hover { background: #334155; color: #e2e8f0; }
        .loading {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 3rem;
          gap: 1rem;
          color: #64748b;
        }
        .spinner {
          width: 32px; height: 32px;
          border: 3px solid #334155;
          border-top-color: #3b82f6;
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }
        @keyframes spin { to { transform: rotate(360deg); } }
        .error-banner {
          background: rgba(239,68,68,0.1);
          border: 1px solid rgba(239,68,68,0.3);
          border-radius: 8px;
          padding: 0.75rem 1rem;
          margin-bottom: 1rem;
          color: #fca5a5;
        }
        .global-limit-warning {
          background: rgba(239,68,68,0.15);
          border: 1px solid rgba(239,68,68,0.4);
          border-radius: 8px;
          padding: 1rem;
          margin-bottom: 1.5rem;
          color: #fca5a5;
        }
        .limits-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
          gap: 1rem;
          margin-bottom: 2rem;
        }
        .limit-card {
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 12px;
          padding: 1.25rem;
        }
        .limit-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 0.75rem;
        }
        .limit-label { font-weight: 600; font-size: 0.95rem; }
        .limit-endpoint {
          font-size: 0.7rem;
          color: #64748b;
          font-family: monospace;
          background: #0f172a;
          padding: 0.2rem 0.4rem;
          border-radius: 4px;
        }
        .limit-numbers {
          display: flex;
          align-items: baseline;
          gap: 0.25rem;
          margin-bottom: 0.75rem;
        }
        .limit-current { font-size: 2rem; font-weight: 700; line-height: 1; }
        .limit-separator { color: #475569; font-size: 1.5rem; }
        .limit-max { font-size: 1.25rem; color: #94a3b8; }
        .limit-unit { font-size: 0.75rem; color: #64748b; margin-right: 0.5rem; }
        .progress-bar-container {
          height: 6px;
          background: #0f172a;
          border-radius: 3px;
          overflow: hidden;
          margin-bottom: 0.5rem;
        }
        .progress-bar {
          height: 100%;
          border-radius: 3px;
          transition: width 0.5s ease;
        }
        .limit-footer {
          display: flex;
          justify-content: space-between;
          font-size: 0.75rem;
        }
        .remaining { font-weight: 500; }
        .reset-time { color: #64748b; }
        .info-section {
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 12px;
          padding: 1.25rem;
        }
        .info-section h3 { margin: 0 0 0.75rem; font-size: 1rem; }
        .info-section ul {
          margin: 0;
          padding-right: 1.25rem;
          color: #94a3b8;
          font-size: 0.875rem;
          line-height: 1.8;
        }
      `}</style>
    </div>
  );
}
