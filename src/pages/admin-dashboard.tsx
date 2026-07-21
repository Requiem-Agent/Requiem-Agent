// admin-dashboard.tsx — S10-04: Admin Dashboard
// لوحة تحكم للمسؤولين: المستخدمون، الإحصائيات، صحة النظام

import { useQuery } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL ?? "http://localhost:7860";

interface SystemStats {
  total_users: number;
  active_users_24h: number;
  total_conversations: number;
  total_messages: number;
  total_llm_calls: number;
  total_tokens_used: number;
  avg_response_time_ms: number;
  error_rate_pct: number;
  uptime_hours: number;
}

interface UserRow {
  id: string;
  email: string;
  created_at: string;
  last_active_at: string | null;
  conversation_count: number;
  message_count: number;
  is_active: boolean;
}

interface SystemHealth {
  status: "healthy" | "degraded" | "down";
  db_connected: boolean;
  db_pool_utilization_pct: number;
  memory_usage_mb: number;
  cpu_usage_pct: number;
  uptime_seconds: number;
  version: string;
}

async function fetchAdminStats(): Promise<{ success: boolean; data: SystemStats }> {
  const res = await fetch(`${API_BASE}/api/admin/stats`, { credentials: "include" });
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}

async function fetchAdminUsers(page: number): Promise<{ success: boolean; data: { users: UserRow[]; total: number } }> {
  const res = await fetch(`${API_BASE}/api/admin/users?page=${page}&per_page=20`, { credentials: "include" });
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}

async function fetchSystemHealth(): Promise<{ success: boolean; data: SystemHealth }> {
  const res = await fetch(`${API_BASE}/healthz`, { credentials: "include" });
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}

function StatCard({ label, value, unit, color }: { label: string; value: string | number; unit?: string; color?: string }) {
  return (
    <div className="stat-card">
      <div className="stat-value" style={{ color: color ?? "#e2e8f0" }}>
        {typeof value === "number" ? value.toLocaleString("ar-DZ") : value}
        {unit && <span className="stat-unit">{unit}</span>}
      </div>
      <div className="stat-label">{label}</div>
    </div>
  );
}

function HealthBadge({ status }: { status: SystemHealth["status"] }) {
  const config = {
    healthy: { color: "#10b981", label: "سليم", icon: "✅" },
    degraded: { color: "#f59e0b", label: "متدهور", icon: "⚠️" },
    down: { color: "#ef4444", label: "معطّل", icon: "🔴" },
  }[status];

  return (
    <span className="health-badge" style={{ color: config.color, borderColor: config.color }}>
      {config.icon} {config.label}
    </span>
  );
}

export default function AdminDashboard() {
  const { data: statsData, isLoading: statsLoading } = useQuery({
    queryKey: ["admin-stats"],
    queryFn: fetchAdminStats,
    refetchInterval: 30_000,
  });

  const { data: usersData, isLoading: usersLoading } = useQuery({
    queryKey: ["admin-users", 1],
    queryFn: () => fetchAdminUsers(1),
    staleTime: 60_000,
  });

  const { data: healthData } = useQuery({
    queryKey: ["system-health"],
    queryFn: fetchSystemHealth,
    refetchInterval: 15_000,
  });

  const stats = statsData?.data;
  const users = usersData?.data;
  const health = healthData?.data;

  return (
    <div className="admin-page" dir="rtl">
      <div className="page-header">
        <div>
          <h1 className="page-title">🛡️ لوحة الإدارة</h1>
          <p className="page-subtitle">إحصائيات النظام وإدارة المستخدمين</p>
        </div>
        {health && <HealthBadge status={health.status} />}
      </div>

      {/* System Health */}
      {health && (
        <div className="health-section">
          <h2 className="section-title">🔧 صحة النظام</h2>
          <div className="health-grid">
            <div className={`health-item ${health.db_connected ? "ok" : "error"}`}>
              <span>{health.db_connected ? "✅" : "❌"}</span>
              <span>قاعدة البيانات</span>
            </div>
            <div className="health-item ok">
              <span>🔄</span>
              <span>Pool: {health.db_pool_utilization_pct.toFixed(0)}%</span>
            </div>
            <div className="health-item ok">
              <span>💾</span>
              <span>RAM: {health.memory_usage_mb.toFixed(0)} MB</span>
            </div>
            <div className="health-item ok">
              <span>⚡</span>
              <span>CPU: {health.cpu_usage_pct.toFixed(1)}%</span>
            </div>
            <div className="health-item ok">
              <span>⏱️</span>
              <span>Uptime: {Math.floor(health.uptime_seconds / 3600)}h</span>
            </div>
            <div className="health-item ok">
              <span>🏷️</span>
              <span>v{health.version}</span>
            </div>
          </div>
        </div>
      )}

      {/* Stats */}
      <div className="stats-section">
        <h2 className="section-title">📊 إحصائيات عامة</h2>
        {statsLoading ? (
          <div className="loading">جارٍ التحميل...</div>
        ) : stats ? (
          <div className="stats-grid">
            <StatCard label="إجمالي المستخدمين" value={stats.total_users} />
            <StatCard label="نشطون (24h)" value={stats.active_users_24h} color="#10b981" />
            <StatCard label="المحادثات" value={stats.total_conversations} />
            <StatCard label="الرسائل" value={stats.total_messages} />
            <StatCard label="LLM Calls" value={stats.total_llm_calls} />
            <StatCard label="Tokens المستخدَمة" value={stats.total_tokens_used} />
            <StatCard label="متوسط الاستجابة" value={stats.avg_response_time_ms.toFixed(0)} unit="ms" />
            <StatCard
              label="معدل الأخطاء"
              value={stats.error_rate_pct.toFixed(2)}
              unit="%"
              color={stats.error_rate_pct > 5 ? "#ef4444" : "#10b981"}
            />
          </div>
        ) : null}
      </div>

      {/* Users table */}
      <div className="users-section">
        <h2 className="section-title">👥 المستخدمون ({users?.total ?? 0})</h2>
        {usersLoading ? (
          <div className="loading">جارٍ التحميل...</div>
        ) : users ? (
          <div className="table-container">
            <table className="users-table">
              <thead>
                <tr>
                  <th>البريد الإلكتروني</th>
                  <th>تاريخ التسجيل</th>
                  <th>آخر نشاط</th>
                  <th>المحادثات</th>
                  <th>الرسائل</th>
                  <th>الحالة</th>
                </tr>
              </thead>
              <tbody>
                {users.users.map((user) => (
                  <tr key={user.id}>
                    <td className="email-cell">{user.email}</td>
                    <td>{new Date(user.created_at).toLocaleDateString("ar-DZ")}</td>
                    <td>{user.last_active_at ? new Date(user.last_active_at).toLocaleDateString("ar-DZ") : "—"}</td>
                    <td>{user.conversation_count}</td>
                    <td>{user.message_count}</td>
                    <td>
                      <span className={`status-pill ${user.is_active ? "active" : "inactive"}`}>
                        {user.is_active ? "نشط" : "معطّل"}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : null}
      </div>

      <style>{`
        .admin-page {
          max-width: 1200px;
          margin: 0 auto;
          padding: 2rem 1rem;
          color: #e2e8f0;
          font-family: 'Segoe UI', Tahoma, sans-serif;
        }
        .page-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 2rem;
        }
        .page-title { font-size: 1.75rem; font-weight: 700; margin: 0 0 0.25rem; }
        .page-subtitle { color: #94a3b8; margin: 0; }
        .health-badge {
          padding: 0.4rem 0.9rem;
          border-radius: 999px;
          border: 1px solid;
          font-size: 0.875rem;
          font-weight: 500;
        }
        .section-title { font-size: 1.1rem; font-weight: 600; margin: 0 0 1rem; color: #94a3b8; }
        .health-section, .stats-section, .users-section {
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 12px;
          padding: 1.5rem;
          margin-bottom: 1.5rem;
        }
        .health-grid {
          display: flex;
          flex-wrap: wrap;
          gap: 0.75rem;
        }
        .health-item {
          display: flex;
          align-items: center;
          gap: 0.4rem;
          background: #0f172a;
          padding: 0.4rem 0.75rem;
          border-radius: 8px;
          font-size: 0.85rem;
          border: 1px solid #334155;
        }
        .health-item.ok { border-color: rgba(16,185,129,0.3); }
        .health-item.error { border-color: rgba(239,68,68,0.3); color: #f87171; }
        .stats-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
          gap: 1rem;
        }
        .stat-card {
          background: #0f172a;
          border: 1px solid #334155;
          border-radius: 10px;
          padding: 1rem;
          text-align: center;
        }
        .stat-value { font-size: 1.5rem; font-weight: 700; line-height: 1.2; }
        .stat-unit { font-size: 0.75rem; color: #94a3b8; margin-right: 0.25rem; }
        .stat-label { font-size: 0.75rem; color: #64748b; margin-top: 0.25rem; }
        .loading { color: #64748b; text-align: center; padding: 2rem; }
        .table-container { overflow-x: auto; }
        .users-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 0.875rem;
        }
        .users-table th {
          text-align: right;
          padding: 0.75rem;
          color: #64748b;
          border-bottom: 1px solid #334155;
          font-weight: 500;
        }
        .users-table td {
          padding: 0.75rem;
          border-bottom: 1px solid #1e293b;
          color: #cbd5e1;
        }
        .users-table tr:hover td { background: rgba(255,255,255,0.02); }
        .email-cell { font-family: monospace; color: #93c5fd; }
        .status-pill {
          padding: 0.2rem 0.6rem;
          border-radius: 999px;
          font-size: 0.75rem;
          font-weight: 500;
        }
        .status-pill.active { background: rgba(16,185,129,0.15); color: #34d399; }
        .status-pill.inactive { background: rgba(107,114,128,0.15); color: #9ca3af; }
      `}</style>
    </div>
  );
}
