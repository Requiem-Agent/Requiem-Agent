import React, { useState } from "react";
import { useAuth } from "@/hooks/use-auth";
import { Logo } from "@/components/logo";

// ── Shared styles ──────────────────────────────────────────────────────────────
const apiBase = import.meta.env.VITE_API_URL || "";

const screenStyle: React.CSSProperties = {
  display: "flex", height: "100dvh", width: "100%",
  alignItems: "center", justifyContent: "center",
  background: "#FFFFFF", flexDirection: "column", gap: "24px",
  padding: "24px", fontFamily: "'Inter','Cairo','Noto Sans Arabic',system-ui,sans-serif",
};

const inputStyle: React.CSSProperties = {
  background: "#F9FAFB", border: "1.5px solid #E5E5E5",
  borderRadius: "12px", padding: "14px 16px", fontSize: "15px",
  color: "#000000", outline: "none", width: "100%", boxSizing: "border-box",
  transition: "border-color 0.2s, box-shadow 0.2s",
};

const buttonStyle: React.CSSProperties = {
  background: "#000000", color: "#FFFFFF", border: "none",
  borderRadius: "12px", padding: "14px", fontSize: "15px",
  fontWeight: 600, cursor: "pointer", marginTop: "6px", width: "100%",
  transition: "transform 0.15s, opacity 0.15s",
  letterSpacing: "0.3px",
};

const errorStyle: React.CSSProperties = {
  color: "#DC2626", fontSize: "13px", margin: "6px 0 0", textAlign: "center" as const,
  background: "#FEF2F2", padding: "10px 14px", borderRadius: "10px",
  border: "1px solid #FECACA",
};

const linkStyle: React.CSSProperties = {
  color: "#000000", textDecoration: "none", fontSize: "13px",
  background: "none", border: "none", cursor: "pointer", padding: "8px 12px",
  fontWeight: 600, borderRadius: "8px", transition: "background 0.15s",
};

// ── Loading screen ────────────────────────────────────────────────────────────
function LoadingScreen() {
  return (
    <div style={screenStyle}>
      <Logo size={72} variant="full" />
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "12px" }}>
        <div style={{ width: "160px", height: "3px", background: "#F3F4F6", borderRadius: "2px", overflow: "hidden" }}>
          <div style={{
            height: "100%", background: "#000000", borderRadius: "2px",
            animation: "ra-progress 2s ease-in-out infinite",
          }} />
        </div>
        <p style={{ color: "#737373", fontSize: "13px", margin: 0, letterSpacing: "0.3px" }}>
          جاري الاتصال…
        </p>
      </div>
      <style>{`
        @keyframes ra-progress { 0% { width: 5%; } 50% { width: 72%; } 100% { width: 94%; } }
      `}</style>
    </div>
  );
}

// ── Register screen ───────────────────────────────────────────────────────────
function RegisterScreen({ onBack, onRegistered }: { onBack: () => void; onRegistered: () => void }) {
  const { login } = useAuth();
  const [step, setStep] = useState<1 | 2>(1);
  const [apiKey, setApiKey] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const verifyKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim().startsWith("ppcrn_")) {
      setError("أدخل مفتاح PopCornصالحاً (يبدأ بـ ppcrn_)");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const res = await fetch(`${apiBase}/api/auth/validate-key`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: apiKey.trim() }),
      });
      const data = await res.json();
      if (!res.ok || !data.valid) {
        setError(data.error || "المفتاح غير صالح");
        setLoading(false);
        return;
      }
      if (data.account_exists) {
        setError("هذا المفتاح مرتبط بحساب موجود — سجّل الدخول بدلاً من ذلك");
        setLoading(false);
        return;
      }
      setStep(2);
    } catch {
      setError("تعذر الوصول للخادم — حاول مجدداً");
    }
    setLoading(false);
  };

  const createAccount = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim() || !password) {
      setError("أدخل اسم المستخدم وكلمة المرور");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const res = await fetch(`${apiBase}/api/auth/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: apiKey.trim(), username: username.trim(), password }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "فشل إنشاء الحساب");
        setLoading(false);
        return;
      }
      const result = await login(username.trim(), password);
      if (!result.success) {
        setError(result.error || "تم إنشاء الحساب — سجّل الدخول يدوياً");
        setLoading(false);
        return;
      }
      onRegistered();
    } catch {
      setError("تعذر الوصول للخادم — حاول مجدداً");
    }
    setLoading(false);
  };

  return (
    <div dir="rtl" style={screenStyle}>
      <Logo size={72} variant="full" />
      <div style={{ textAlign: "center" }}>
        <h1 style={{ fontSize: "22px", fontWeight: 700, color: "#000000", margin: "0 0 6px" }}>
          {step === 1 ? "تسجيل جديد" : "إنشاء حساب"}
        </h1>
        <p style={{ color: "#737373", fontSize: "14px", margin: 0, lineHeight: 1.5 }}>
          {step === 1 ? "أدخل مفتاح PopCorn للتحقق" : "اختر اسم مستخدم وكلمة مرور"}
        </p>
      </div>
      <form onSubmit={step === 1 ? verifyKey : createAccount} style={{
        width: "100%", maxWidth: "360px", display: "flex", flexDirection: "column", gap: "14px",
      }}>
        {step === 1 ? (
          <input
            type="text"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="مفتاح PopCorn (ppcrn_...)"
            style={inputStyle}
          />
        ) : (
          <>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="اسم المستخدم"
              autoComplete="username"
              style={inputStyle}
            />
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="كلمة المرور"
              autoComplete="new-password"
              style={inputStyle}
            />
          </>
        )}
        {error && <p style={errorStyle}>{error}</p>}
        <button type="submit" disabled={loading} style={{ ...buttonStyle, opacity: loading ? 0.6 : 1 }}>
          {loading ? "…جاري" : step === 1 ? "تحقق من المفتاح" : "إنشاء الحساب"}
        </button>
      </form>
      <button onClick={onBack} style={linkStyle}>← العودة لتسجيل الدخول</button>
    </div>
  );
}

// ── Forgot password screen ────────────────────────────────────────────────────
function ForgotPasswordScreen({ onBack }: { onBack: () => void }) {
  const [apiKey, setApiKey] = useState("");
  const [username, setUsername] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [error, setError] = useState("");
  const [done, setDone] = useState(false);
  const [loading, setLoading] = useState(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim() || !username.trim() || !newPassword) {
      setError("أدخل جميع الحقول المطلوبة");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const res = await fetch(`${apiBase}/api/auth/reset-password`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: apiKey.trim(), username: username.trim(), new_password: newPassword }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "فشلت العملية");
        setLoading(false);
        return;
      }
      setDone(true);
    } catch {
      setError("تعذر الوصول للخادم — حاول مجدداً");
    }
    setLoading(false);
  };

  return (
    <div dir="rtl" style={screenStyle}>
      <Logo size={72} variant="full" />
      <div style={{ textAlign: "center" }}>
        <h1 style={{ fontSize: "22px", fontWeight: 700, color: "#000000", margin: "0 0 6px" }}>
          استعادة كلمة المرور
        </h1>
        <p style={{ color: "#737373", fontSize: "14px", margin: 0, lineHeight: 1.5 }}>
          {done ? "تم التحديث بنجاح" : "أدخل المفتاح واسم المستخدم وكلمة المرور الجديدة"}
        </p>
      </div>
      {done ? (
        <button onClick={onBack} style={buttonStyle}>العودة لتسجيل الدخول</button>
      ) : (
        <form onSubmit={submit} style={{
          width: "100%", maxWidth: "360px", display: "flex", flexDirection: "column", gap: "14px",
        }}>
          <input type="text" value={apiKey} onChange={(e) => setApiKey(e.target.value)} placeholder="مفتاح PopCorn (ppcrn_...)" style={inputStyle} />
          <input type="text" value={username} onChange={(e) => setUsername(e.target.value)} placeholder="اسم المستخدم" autoComplete="username" style={inputStyle} />
          <input type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} placeholder="كلمة المرور الجديدة" autoComplete="new-password" style={inputStyle} />
          {error && <p style={errorStyle}>{error}</p>}
          <button type="submit" disabled={loading} style={{ ...buttonStyle, opacity: loading ? 0.6 : 1 }}>
            {loading ? "…جاري" : "تحديث كلمة المرور"}
          </button>
        </form>
      )}
      {!done && <button onClick={onBack} style={linkStyle}>← العودة لتسجيل الدخول</button>}
    </div>
  );
}

// ── Login screen ──────────────────────────────────────────────────────────────
function LoginScreen() {
  const { login } = useAuth();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [view, setView] = useState<"login" | "register" | "forgot">("login");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim() || !password) {
      setError("أدخل اسم المستخدم وكلمة المرور");
      return;
    }
    setLoading(true);
    setError("");
    const result = await login(username.trim(), password);
    if (!result.success) {
      setError(result.error || "فشل تسجيل الدخول");
    }
    setLoading(false);
  };

  if (view === "register") return <RegisterScreen onBack={() => setView("login")} onRegistered={() => setView("login")} />;
  if (view === "forgot") return <ForgotPasswordScreen onBack={() => setView("login")} />;

  return (
    <div dir="rtl" style={screenStyle}>
      <Logo size={80} variant="full" />
      <div style={{ textAlign: "center", marginTop: "4px" }}>
        <h1 style={{ fontSize: "24px", fontWeight: 800, color: "#000000", margin: "0 0 8px", letterSpacing: "-0.5px" }}>
          بوب كورن ستوديو
        </h1>
        <p style={{ color: "#737373", fontSize: "14px", margin: 0, lineHeight: 1.5 }}>
          سجّل الدخول بالحساب المُنشأ عبر البوت
        </p>
      </div>
      <form onSubmit={handleSubmit} style={{
        width: "100%", maxWidth: "360px", display: "flex", flexDirection: "column", gap: "14px",
      }}>
        <input
          type="text"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="اسم المستخدم"
          autoComplete="username"
          style={inputStyle}
          onFocus={(e) => { e.target.style.borderColor = "#000000"; e.target.style.boxShadow = "0 0 0 3px rgba(0,0,0,0.08)"; }}
          onBlur={(e) => { e.target.style.borderColor = "#E5E5E5"; e.target.style.boxShadow = "none"; }}
        />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="كلمة المرور"
          autoComplete="current-password"
          style={inputStyle}
          onFocus={(e) => { e.target.style.borderColor = "#000000"; e.target.style.boxShadow = "0 0 0 3px rgba(0,0,0,0.08)"; }}
          onBlur={(e) => { e.target.style.borderColor = "#E5E5E5"; e.target.style.boxShadow = "none"; }}
        />
        {error && <p style={errorStyle}>{error}</p>}
        <button
          type="submit"
          disabled={loading}
          style={{ ...buttonStyle, opacity: loading ? 0.6 : 1 }}
          onMouseEnter={(e) => { if (!loading) e.currentTarget.style.transform = "scale(0.98)"; }}
          onMouseLeave={(e) => { e.currentTarget.style.transform = "scale(1)"; }}
        >
          {loading ? "…جاري الدخول" : "تسجيل الدخول"}
        </button>
      </form>

      <div style={{ display: "flex", gap: "8px", justifyContent: "center", fontSize: "13px" }}>
        <button onClick={() => setView("register")} style={linkStyle} onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }} onMouseLeave={(e) => { e.currentTarget.style.background = "none"; }}>
          إنشاء حساب جديد
        </button>
        <span style={{ color: "#E5E5E5" }}>|</span>
        <button onClick={() => setView("forgot")} style={{ ...linkStyle, color: "#737373" }} onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }} onMouseLeave={(e) => { e.currentTarget.style.background = "none"; }}>
          نسيت كلمة المرور؟
        </button>
      </div>
    </div>
  );
}

// ── AuthGuard ─────────────────────────────────────────────────────────────────
export default function AuthGuard({ children }: { children: React.ReactNode }) {
  const { user, isLoading } = useAuth();

  if (isLoading) return <LoadingScreen />;
  if (!user) return <LoginScreen />;

  return <>{children}</>;
}
