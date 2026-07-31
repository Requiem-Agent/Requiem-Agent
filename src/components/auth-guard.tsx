import React, { useState } from "react";
import { useAuth } from "@/hooks/use-auth";

// ── Loading screen ────────────────────────────────────────────────────────────
function LoadingScreen() {
  return (
    <div style={{
      display: "flex", height: "100dvh", width: "100%",
      alignItems: "center", justifyContent: "center",
      background: "hsl(240 7% 6%)", flexDirection: "column", gap: "20px",
      fontFamily: "'Inter','Cairo','Noto Sans Arabic',system-ui,sans-serif",
    }}>
      <div style={{
        height: "52px", width: "52px", borderRadius: "14px",
        background: "hsl(262 83% 62% / 0.12)", border: "1px solid hsl(262 83% 62% / 0.25)",
        display: "flex", alignItems: "center", justifyContent: "center",
        animation: "ra-float 3s ease-in-out infinite",
      }}>
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="hsl(262 83% 65%)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          <path d="M9 12l2 2 4-4"/>
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

// ── Login screen ──────────────────────────────────────────────────────────────
function LoginScreen() {
  const { login } = useAuth();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

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

  return (
    <div dir="rtl" style={{
      display: "flex", height: "100dvh", width: "100%",
      alignItems: "center", justifyContent: "center",
      background: "hsl(240 7% 6%)", flexDirection: "column", gap: "20px",
      padding: "24px", fontFamily: "'Inter','Cairo','Noto Sans Arabic',system-ui,sans-serif",
    }}>
      {/* Logo */}
      <div style={{
        height: "60px", width: "60px", borderRadius: "16px",
        background: "hsl(262 83% 62% / 0.12)", border: "1px solid hsl(262 83% 62% / 0.25)",
        display: "flex", alignItems: "center", justifyContent: "center",
      }}>
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="hsl(262 83% 65%)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          <path d="M9 12l2 2 4-4"/>
        </svg>
      </div>

      <div style={{ textAlign: "center" }}>
        <h1 style={{ fontSize: "20px", fontWeight: 700, color: "#fff", margin: "0 0 6px" }}>
          Requiem Agent
        </h1>
        <p style={{ color: "hsl(240 5% 45%)", fontSize: "13px", margin: 0 }}>
          Sign in to continue
        </p>
      </div>

      {/* Login form */}
      <form onSubmit={handleSubmit} style={{
        width: "100%", maxWidth: "340px", display: "flex", flexDirection: "column", gap: "12px",
      }}>
        <input
          type="text"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="Username"
          autoComplete="username"
          style={{
            background: "hsl(240 6% 10%)", border: "1px solid hsl(240 6% 16%)",
            borderRadius: "10px", padding: "12px 14px", fontSize: "14px",
            color: "#fff", outline: "none", width: "100%", boxSizing: "border-box",
          }}
        />
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Password"
          autoComplete="current-password"
          style={{
            background: "hsl(240 6% 10%)", border: "1px solid hsl(240 6% 16%)",
            borderRadius: "10px", padding: "12px 14px", fontSize: "14px",
            color: "#fff", outline: "none", width: "100%", boxSizing: "border-box",
          }}
        />
        {error && (
          <p style={{ color: "hsl(0 83% 62%)", fontSize: "12px", margin: "4px 0 0" }}>
            {error}
          </p>
        )}
        <button
          type="submit"
          disabled={loading}
          style={{
            background: "hsl(262 83% 62%)", color: "#fff", border: "none",
            borderRadius: "10px", padding: "12px", fontSize: "14px",
            fontWeight: 600, cursor: "pointer", marginTop: "4px",
            opacity: loading ? 0.6 : 1,
          }}
        >
          {loading ? "Signing in..." : "Sign In"}
        </button>
      </form>
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
