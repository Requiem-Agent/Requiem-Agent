import React, { useState } from "react";
import { useAuth } from "@/hooks/use-auth";

// ── Shared styles ──────────────────────────────────────────────────────────────
const apiBase = import.meta.env.VITE_API_URL || "";

const screenStyle: React.CSSProperties = {
  display: "flex", height: "100dvh", width: "100%",
  alignItems: "center", justifyContent: "center",
  background: "hsl(240 7% 6%)", flexDirection: "column", gap: "18px",
  padding: "24px", fontFamily: "'Inter','Cairo','Noto Sans Arabic',system-ui,sans-serif",
};

const inputStyle: React.CSSProperties = {
  background: "hsl(240 6% 10%)", border: "1px solid hsl(240 6% 16%)",
  borderRadius: "10px", padding: "12px 14px", fontSize: "14px",
  color: "#fff", outline: "none", width: "100%", boxSizing: "border-box",
};

const buttonStyle: React.CSSProperties = {
  background: "hsl(262 83% 62%)", color: "#fff", border: "none",
  borderRadius: "10px", padding: "12px", fontSize: "14px",
  fontWeight: 600, cursor: "pointer", marginTop: "4px", width: "100%",
};

const errorStyle: React.CSSProperties = {
  color: "hsl(0 83% 62%)", fontSize: "12px", margin: "4px 0 0", textAlign: "center" as const,
};

const linkStyle: React.CSSProperties = {
  color: "hsl(262 83% 65%)", textDecoration: "none", fontSize: "12px",
  background: "none", border: "none", cursor: "pointer", padding: 0,
};

function Logo() {
  return (
    <div style={{
      height: "60px", width: "60px", borderRadius: "16px",
      background: "hsl(262 83% 62% / 0.12)", border: "1px solid hsl(262 83% 62% / 0.25)",
      display: "flex", alignItems: "center", justifyContent: "center",
    }}>
      <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="hsl(262 83% 65%)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M6 10h12l-1.2 10a2 2 0 0 1-2 1.8H9.2a2 2 0 0 1-2-1.8L6 10z"/>
        <circle cx="8.5" cy="7.5" r="1.6"/>
        <circle cx="12" cy="6" r="1.8"/>
        <circle cx="15.5" cy="7.5" r="1.6"/>
      </svg>
    </div>
  );
}

function ScreenTitle({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <div style={{ textAlign: "center" }}>
      <h1 style={{ fontSize: "20px", fontWeight: 700, color: "#fff", margin: "0 0 6px" }}>
        {title}
      </h1>
      <p style={{ color: "hsl(240 5% 45%)", fontSize: "13px", margin: 0 }}>
        {subtitle}
      </p>
    </div>
  );
}

// ── Loading screen ────────────────────────────────────────────────────────────
function LoadingScreen() {
  return (
    <div style={screenStyle}>
      <div style={{
        height: "52px", width: "52px", borderRadius: "14px",
        background: "hsl(262 83% 62% / 0.12)", border: "1px solid hsl(262 83% 62% / 0.25)",
        display: "flex", alignItems: "center", justifyContent: "center",
        animation: "ra-float 3s ease-in-out infinite",
      }}>
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="hsl(262 83% 65%)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M6 10h12l-1.2 10a2 2 0 0 1-2 1.8H9.2a2 2 0 0 1-2-1.8L6 10z"/>
          <circle cx="8.5" cy="7.5" r="1.6"/>
          <circle cx="12" cy="6" r="1.8"/>
          <circle cx="15.5" cy="7.5" r="1.6"/>
        </svg>
      </div>
      <div style={{ width: "140px", height: "2px", background: "hsl(240 6% 14%)", borderRadius: "1px", overflow: "hidden" }}>
        <div style={{
          height: "100%",
          background: "linear-gradient(90deg, hsl(262 83% 62%), hsl(188 94% 38%))",
          animation: "ra-progress 2s ease-in-out infinite",
          borderRadius: "1px",
        }} />
      </div>
      <p style={{ color: "hsl(240 5% 45%)", fontSize: "12px", letterSpacing: "0.5px", margin: 0 }}>
        Connecting…
      </p>
      <style>{`
        @keyframes ra-float { 0%, 100% { transform: translateY(0px); } 50% { transform: translateY(-5px); } }
        @keyframes ra-progress { 0% { width: 5%; } 50% { width: 72%; } 100% { width: 94%; } }
      `}</style>
    </div>
  );
}

// ── Register screen (2 steps, stays inside the Mini App) ──────────────────────
function RegisterScreen({ onBack, onRegistered }: { onBack: () => void; onRegistered: () => void }) {
  const { login } = useAuth();
  const [step, setStep] = useState<1 | 2>(1);
  const [apiKey, setApiKey] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  // Step 1: verify the PopCorn key via the gateway
  const verifyKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim().startsWith("ppcrn_")) {
      setError("أدخل مفتاح PopCorn صالحاً (يبدأ بـ ppcrn_)");
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
        setError("هذا المفتاح مرتبط بحساب موجود بالفعل — سجّل الدخول بدلاً من ذلك");
        setLoading(false);
        return;
      }
      setStep(2);
    } catch {
      setError("تعذر الوصول للخادم — حاول مجدداً");
    }
    setLoading(false);
  };

  // Step 2: create account + auto sign-in
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
      // auto sign-in with the new credentials
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
      <Logo />
      <ScreenTitle
        title={step === 1 ? "تسجيل جديد" : "إنشاء حساب"}
        subtitle={step === 1 ? "أدخل مفتاح PopCorn الخاص بك للتحقق" : "اختر اسم مستخدم وكلمة مرور"}
      />
      <form onSubmit={step === 1 ? verifyKey : createAccount} style={{
        width: "100%", maxWidth: "340px", display: "flex", flexDirection: "column", gap: "12px",
      }}>
        {step === 1 ? (
          <input
            type="text"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="PopCorn API Key (ppcrn_...)"
            style={inputStyle}
          />
        ) : (
          <>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="اسم المستخدم (3-32 حرفاً)"
              autoComplete="username"
              style={inputStyle}
            />
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="كلمة المرور (8+ أحرف، حرف كبير ورقم)"
              autoComplete="new-password"
              style={inputStyle}
            />
          </>
        )}
        {error && <p style={errorStyle}>{error}</p>}
        <button type="submit" disabled={loading} style={{ ...buttonStyle, opacity: loading ? 0.6 : 1 }}>
          {loading ? "...جاري" : step === 1 ? "تحقق من المفتاح" : "إنشاء الحساب"}
        </button>
      </form>
      <div style={{ display: "flex", gap: "16px", justifyContent: "center" }}>
        <button onClick={onBack} style={linkStyle}>عودة لتسجيل الدخول</button>
      </div>
    </div>
  );
}

// ── Forgot password screen (stays inside the Mini App) ────────────────────────
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
      setError("أدخل جميع الحقول");
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
        setError(data.error || "فشلت الاستعادة");
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
      <Logo />
      <ScreenTitle
        title="استعادة كلمة المرور"
        subtitle={done ? "تم تحديث كلمة المرور بنجاح" : "أدخل المفتاح واسم المستخدم وكلمة مرور جديدة"}
      />
      {done ? (
        <button onClick={onBack} style={buttonStyle}>العودة لتسجيل الدخول</button>
      ) : (
        <form onSubmit={submit} style={{
          width: "100%", maxWidth: "340px", display: "flex", flexDirection: "column", gap: "12px",
        }}>
          <input
            type="text"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="PopCorn API Key (ppcrn_...)"
            style={inputStyle}
          />
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
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            placeholder="كلمة المرور الجديدة"
            autoComplete="new-password"
            style={inputStyle}
          />
          {error && <p style={errorStyle}>{error}</p>}
          <button type="submit" disabled={loading} style={{ ...buttonStyle, opacity: loading ? 0.6 : 1 }}>
            {loading ? "...جاري" : "تحديث كلمة المرور"}
          </button>
        </form>
      )}
      {!done && (
        <div style={{ display: "flex", gap: "16px", justifyContent: "center" }}>
          <button onClick={onBack} style={linkStyle}>عودة لتسجيل الدخول</button>
        </div>
      )}
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
      setError("Please enter username and password");
      return;
    }
    setLoading(true);
    setError("");
    const result = await login(username.trim(), password);
    if (!result.success) {
      setError(result.error || "Login failed");
    }
    setLoading(false);
  };

  if (view === "register") {
    return <RegisterScreen onBack={() => setView("login")} onRegistered={() => setView("login")} />;
  }
  if (view === "forgot") {
    return <ForgotPasswordScreen onBack={() => setView("login")} />;
  }

  return (
    <div dir="rtl" style={screenStyle}>
      <Logo />
      <ScreenTitle
        title="PopCorn AI Studio"
        subtitle="سجّل الدخول بحساب PopCorn الخاص بك"
      />
      <form onSubmit={handleSubmit} style={{
        width: "100%", maxWidth: "340px", display: "flex", flexDirection: "column", gap: "12px",
      }}>
        <input
          type="text"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="Username"
          autoComplete="username"
          style={inputStyle}
        />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Password"
          autoComplete="current-password"
          style={inputStyle}
        />
        {error && <p style={errorStyle}>{error}</p>}
        <button
          type="submit"
          disabled={loading}
          style={{ ...buttonStyle, opacity: loading ? 0.6 : 1 }}
        >
          {loading ? "Signing in..." : "Sign In"}
        </button>
      </form>

      {/* كل شيء يتم داخل التطبيق المصغر — لا نوافذ منبقة خارجية */}
      <div style={{ display: "flex", gap: "16px", justifyContent: "center", fontSize: "12px" }}>
        <button onClick={() => setView("register")} style={linkStyle}>
          تسجيل جديد
        </button>
        <button onClick={() => setView("forgot")} style={{ ...linkStyle, color: "hsl(240 5% 45%)" }}>
          نسيت كلمة السر
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
