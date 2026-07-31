// api-keys.tsx — S7-05: API Keys Management Page
// صفحة إدارة مفاتيح LLM providers مع مؤشرات الحالة (connected/disconnected)

import { useState } from "react";
import { useApiKeys, useSaveApiKey, useDeleteApiKey } from "../hooks/use-preferences";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type Provider = "anthropic" | "openai" | "gemini" | "mistral";

interface ProviderMeta {
  id: Provider;
  name: string;
  logo: string;
  docsUrl: string;
  keyPrefix: string;
  keyPlaceholder: string;
  color: string;
}

const PROVIDERS: ProviderMeta[] = [
  {
    id: "anthropic",
    name: "Anthropic (Claude)",
    logo: "🤖",
    docsUrl: "https://console.anthropic.com/settings/keys",
    keyPrefix: "sk-ant-",
    keyPlaceholder: "sk-ant-api03-...",
    color: "#d97706",
  },
  {
    id: "openai",
    name: "OpenAI (GPT)",
    logo: "🧠",
    docsUrl: "https://platform.openai.com/api-keys",
    keyPrefix: "sk-",
    keyPlaceholder: "sk-proj-...",
    color: "#10b981",
  },
  {
    id: "gemini",
    name: "Google Gemini",
    logo: "✨",
    docsUrl: "https://aistudio.google.com/app/apikey",
    keyPrefix: "AIza",
    keyPlaceholder: "AIzaSy...",
    color: "#3b82f6",
  },
  {
    id: "mistral",
    name: "Mistral AI",
    logo: "🌪️",
    docsUrl: "https://console.mistral.ai/api-keys/",
    keyPrefix: "",
    keyPlaceholder: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
    color: "#8b5cf6",
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// ProviderCard component
// ─────────────────────────────────────────────────────────────────────────────

interface ProviderCardProps {
  provider: ProviderMeta;
  storedKey?: { id: string; key_hint: string; created_at: string };
  onSave: (provider: Provider, key: string) => Promise<void>;
  onDelete: (keyId: string) => Promise<void>;
}

function ProviderCard({ provider, storedKey, onSave, onDelete }: ProviderCardProps) {
  const [inputKey, setInputKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  const isConnected = !!storedKey;

  const handleSave = async () => {
    if (!inputKey.trim()) {
      setError("الرجاء إدخال المفتاح");
      return;
    }
    setError(null);
    setSaving(true);
    try {
      await onSave(provider.id, inputKey.trim());
      setInputKey("");
      setShowForm(false);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "فشل حفظ المفتاح");
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!storedKey) return;
    setDeleting(true);
    try {
      await onDelete(storedKey.id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "فشل حذف المفتاح");
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div
      className="provider-card"
      style={{ borderLeft: `4px solid ${provider.color}` }}
    >
      {/* Header */}
      <div className="provider-header">
        <div className="provider-identity">
          <span className="provider-logo">{provider.logo}</span>
          <div>
            <h3 className="provider-name">{provider.name}</h3>
            <a
              href={provider.docsUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="provider-docs-link"
            >
              احصل على مفتاح API ↗
            </a>
          </div>
        </div>

        {/* Status badge */}
        <div className={`status-badge ${isConnected ? "connected" : "disconnected"}`}>
          <span className="status-dot" />
          {isConnected ? "متصل" : "غير متصل"}
        </div>
      </div>

      {/* Connected state */}
      {isConnected && storedKey && (
        <div className="key-info">
          <div className="key-hint-row">
            <span className="key-hint-label">المفتاح:</span>
            <code className="key-hint">{storedKey.key_hint}</code>
            <span className="key-date">
              أُضيف {new Date(storedKey.created_at).toLocaleDateString("ar-DZ")}
            </span>
          </div>
          <div className="key-actions">
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => setShowForm(!showForm)}
            >
              {showForm ? "إلغاء" : "تحديث المفتاح"}
            </button>
            <button
              className="btn btn-danger btn-sm"
              onClick={handleDelete}
              disabled={deleting}
            >
              {deleting ? "جارٍ الحذف..." : "حذف"}
            </button>
          </div>
        </div>
      )}

      {/* Add/Update form */}
      {(!isConnected || showForm) && (
        <div className="key-form">
          <div className="input-group">
            <input
              type={showKey ? "text" : "password"}
              value={inputKey}
              onChange={(e) => setInputKey(e.target.value)}
              placeholder={provider.keyPlaceholder}
              className="key-input"
              onKeyDown={(e) => e.key === "Enter" && handleSave()}
            />
            <button
              className="btn btn-icon"
              onClick={() => setShowKey(!showKey)}
              title={showKey ? "إخفاء" : "إظهار"}
            >
              {showKey ? "🙈" : "👁️"}
            </button>
          </div>
          {error && <p className="error-msg">{error}</p>}
          <button
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving || !inputKey.trim()}
          >
            {saving ? "جارٍ الحفظ..." : isConnected ? "تحديث المفتاح" : "حفظ المفتاح"}
          </button>
        </div>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Main page component
// ─────────────────────────────────────────────────────────────────────────────

export default function ApiKeysPage() {
  const { data: storedKeys = [], isLoading, error: loadError } = useApiKeys();
  const saveApiKey = useSaveApiKey();
  const deleteApiKey = useDeleteApiKey();

  const connectedCount = storedKeys.length;
  const totalCount = PROVIDERS.length;

  const handleSave = async (provider: Provider, key: string) => {
    await saveApiKey.mutateAsync({ provider, api_key: key });
  };

  const handleDelete = async (keyId: string) => {
    await deleteApiKey.mutateAsync(keyId);
  };

  if (isLoading) {
    return (
      <div className="page-loading">
        <div className="spinner" />
        <p>جارٍ تحميل مفاتيح API...</p>
      </div>
    );
  }

  return (
    <div className="api-keys-page" dir="rtl">
      {/* Page header */}
      <div className="page-header">
        <div>
          <h1 className="page-title">🔑 مفاتيح API</h1>
          <p className="page-subtitle">
            أضف مفاتيح LLM providers لاستخدامها في المحادثات. تُخزَّن مشفّرة بـ AES-256-GCM.
          </p>
        </div>
        <div className="connection-summary">
          <div className="summary-number" style={{ color: connectedCount > 0 ? "#10b981" : "#6b7280" }}>
            {connectedCount}/{totalCount}
          </div>
          <div className="summary-label">providers متصلة</div>
        </div>
      </div>

      {/* Security notice */}
      <div className="security-notice">
        <span className="security-icon">🔒</span>
        <div>
          <strong>تشفير كامل:</strong> مفاتيحك تُشفَّر بـ AES-256-GCM قبل التخزين.
          لا يمكن لأحد — حتى المطورين — رؤية مفاتيحك بشكل نصي.
        </div>
      </div>

      {/* Error state */}
      {loadError && (
        <div className="error-banner">
          ⚠️ فشل تحميل المفاتيح: {loadError instanceof Error ? loadError.message : "خطأ غير معروف"}
        </div>
      )}

      {/* Provider cards */}
      <div className="providers-grid">
        {PROVIDERS.map((provider) => {
          const stored = storedKeys.find((k) => k.provider === provider.id);
          return (
            <ProviderCard
              key={provider.id}
              provider={provider}
              storedKey={stored}
              onSave={handleSave}
              onDelete={handleDelete}
            />
          );
        })}
      </div>

      {/* Styles */}
      <style>{`
        .api-keys-page {
          max-width: 800px;
          margin: 0 auto;
          padding: 2rem 1rem;
          font-family: 'Segoe UI', Tahoma, sans-serif;
          color: #e2e8f0;
        }
        .page-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 1.5rem;
          gap: 1rem;
        }
        .page-title { font-size: 1.75rem; font-weight: 700; margin: 0 0 0.25rem; }
        .page-subtitle { color: #94a3b8; margin: 0; font-size: 0.9rem; }
        .connection-summary { text-align: center; }
        .summary-number { font-size: 2rem; font-weight: 700; line-height: 1; }
        .summary-label { font-size: 0.75rem; color: #94a3b8; }
        .security-notice {
          display: flex;
          align-items: flex-start;
          gap: 0.75rem;
          background: rgba(16, 185, 129, 0.1);
          border: 1px solid rgba(16, 185, 129, 0.3);
          border-radius: 8px;
          padding: 0.75rem 1rem;
          margin-bottom: 1.5rem;
          font-size: 0.875rem;
          color: #a7f3d0;
        }
        .security-icon { font-size: 1.25rem; flex-shrink: 0; }
        .error-banner {
          background: rgba(239, 68, 68, 0.1);
          border: 1px solid rgba(239, 68, 68, 0.3);
          border-radius: 8px;
          padding: 0.75rem 1rem;
          margin-bottom: 1rem;
          color: #fca5a5;
          font-size: 0.875rem;
        }
        .providers-grid { display: flex; flex-direction: column; gap: 1rem; }
        .provider-card {
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 12px;
          padding: 1.25rem;
          transition: border-color 0.2s;
        }
        .provider-card:hover { border-color: #475569; }
        .provider-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 1rem;
        }
        .provider-identity { display: flex; align-items: center; gap: 0.75rem; }
        .provider-logo { font-size: 1.75rem; }
        .provider-name { font-size: 1rem; font-weight: 600; margin: 0 0 0.2rem; }
        .provider-docs-link { font-size: 0.75rem; color: #60a5fa; text-decoration: none; }
        .provider-docs-link:hover { text-decoration: underline; }
        .status-badge {
          display: flex;
          align-items: center;
          gap: 0.4rem;
          padding: 0.3rem 0.75rem;
          border-radius: 999px;
          font-size: 0.8rem;
          font-weight: 500;
        }
        .status-badge.connected { background: rgba(16,185,129,0.15); color: #34d399; }
        .status-badge.disconnected { background: rgba(107,114,128,0.15); color: #9ca3af; }
        .status-dot {
          width: 8px; height: 8px; border-radius: 50%;
          background: currentColor;
          animation: pulse 2s infinite;
        }
        .status-badge.disconnected .status-dot { animation: none; }
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
        .key-info { margin-bottom: 0.75rem; }
        .key-hint-row {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          margin-bottom: 0.5rem;
          font-size: 0.875rem;
        }
        .key-hint-label { color: #94a3b8; }
        .key-hint {
          background: #0f172a;
          padding: 0.2rem 0.5rem;
          border-radius: 4px;
          font-family: monospace;
          color: #a5f3fc;
          font-size: 0.8rem;
        }
        .key-date { color: #64748b; font-size: 0.75rem; margin-right: auto; }
        .key-actions { display: flex; gap: 0.5rem; }
        .key-form { display: flex; flex-direction: column; gap: 0.5rem; }
        .input-group { display: flex; gap: 0.5rem; }
        .key-input {
          flex: 1;
          background: #0f172a;
          border: 1px solid #334155;
          border-radius: 8px;
          padding: 0.6rem 0.75rem;
          color: #e2e8f0;
          font-family: monospace;
          font-size: 0.875rem;
          outline: none;
          transition: border-color 0.2s;
        }
        .key-input:focus { border-color: #60a5fa; }
        .key-input::placeholder { color: #475569; }
        .error-msg { color: #f87171; font-size: 0.8rem; margin: 0; }
        .btn {
          padding: 0.5rem 1rem;
          border-radius: 8px;
          border: none;
          cursor: pointer;
          font-size: 0.875rem;
          font-weight: 500;
          transition: all 0.2s;
          white-space: nowrap;
        }
        .btn:disabled { opacity: 0.5; cursor: not-allowed; }
        .btn-primary { background: #3b82f6; color: white; }
        .btn-primary:hover:not(:disabled) { background: #2563eb; }
        .btn-secondary { background: #334155; color: #e2e8f0; }
        .btn-secondary:hover:not(:disabled) { background: #475569; }
        .btn-danger { background: rgba(239,68,68,0.15); color: #f87171; border: 1px solid rgba(239,68,68,0.3); }
        .btn-danger:hover:not(:disabled) { background: rgba(239,68,68,0.25); }
        .btn-icon { background: #334155; color: #94a3b8; padding: 0.5rem 0.6rem; }
        .btn-sm { padding: 0.35rem 0.75rem; font-size: 0.8rem; }
        .page-loading {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          min-height: 200px;
          gap: 1rem;
          color: #94a3b8;
        }
        .spinner {
          width: 32px; height: 32px;
          border: 3px solid #334155;
          border-top-color: #3b82f6;
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }
        @keyframes spin { to { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
}
