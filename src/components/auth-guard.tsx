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
      {/* Logo */}
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

      {/* Progress bar */}
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
        @keyframes ra-float {
          0%, 100% { transform: translateY(0px); }
          50% { transform: translateY(-5px); }
        }
        @keyframes ra-progress {
          0% { width: 5%; }
          50% { width: 72%; }
          100% { width: 94%; }
        }
      `}</style>
    </div>
  );
}

// ── Access denied screen ──────────────────────────────────────────────────────
function AccessDeniedScreen({ inTelegram }: { inTelegram: boolean }) {
  if (inTelegram) {
    return (
      <div dir="rtl" style={{
        display: "flex", height: "100dvh", width: "100%",
        alignItems: "center", justifyContent: "center",
        background: "hsl(240 7% 6%)", flexDirection: "column", gap: "16px",
        padding: "32px", textAlign: "center",
        fontFamily: "'Inter','Cairo','Noto Sans Arabic',system-ui,sans-serif",
      }}>
        <div style={{
          height: "60px", width: "60px", borderRadius: "16px",
          background: "hsl(40 83% 62% / 0.1)", border: "1px solid hsl(40 83% 62% / 0.2)",
          display: "flex", alignItems: "center", justifyContent: "center",
        }}>
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="hsl(40 83% 65%)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
        </div>
        <div style={{ maxWidth: "280px" }}>
          <h1 style={{ fontSize: "18px", fontWeight: 700, color: "#fff", margin: "0 0 8px" }}>
            فشل التحقق
          </h1>
          <p style={{ color: "hsl(240 5% 55%)", fontSize: "13px", lineHeight: "1.8", margin: 0 }}>
            تعذر التحقق من هوية تلغرام. حاول إعادة فتح التطبيق.
          </p>
        </div>
        <button onClick={() => window.location.reload()}
          style={{
            background: "hsl(262 83% 62%)", color: "#fff", border: "none",
            borderRadius: "10px", padding: "10px 24px", fontSize: "13px",
            fontWeight: 600, cursor: "pointer", marginTop: "8px",
          }}>
          إعادة المحاولة
        </button>
        <style>{`
          @keyframes ra-float {
            0%, 100% { transform: translateY(0px); }
            50% { transform: translateY(-5px); }
          }
        `}</style>
      </div>
    );
  }
  return (
    <div dir="rtl" style={{
      display: "flex", height: "100dvh", width: "100%",
      alignItems: "center", justifyContent: "center",
      background: "hsl(240 7% 6%)", flexDirection: "column", gap: "16px",
      padding: "32px", textAlign: "center",
      fontFamily: "'Inter','Cairo','Noto Sans Arabic',system-ui,sans-serif",
    }}>
      {/* Shield icon */}
      <div style={{
        height: "60px", width: "60px", borderRadius: "16px",
        background: "hsl(262 83% 62% / 0.1)", border: "1px solid hsl(262 83% 62% / 0.2)",
        display: "flex", alignItems: "center", justifyContent: "center",
        animation: "ra-float 3s ease-in-out infinite",
      }}>
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="hsl(262 83% 65%)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          <path d="M9 12l2 2 4-4"/>
        </svg>
      </div>

      <div style={{ maxWidth: "280px" }}>
        <h1 style={{ fontSize: "18px", fontWeight: 700, color: "#fff", margin: "0 0 8px" }}>
          وصول مقيد
        </h1>
        <p style={{ color: "hsl(240 5% 55%)", fontSize: "13px", lineHeight: "1.8", margin: 0 }}>
          لا يمكن الوصول إلى{" "}
          <span style={{ color: "hsl(262 83% 65%)", fontWeight: 600 }}>Requiem Agent 1</span>
          {" "}إلا من داخل تطبيق تلغرام.
        </p>
      </div>

      <div style={{
        background: "hsl(240 6% 9%)", border: "1px solid hsl(240 6% 14%)",
        borderRadius: "12px", padding: "14px 20px", maxWidth: "260px",
      }}>
        <p style={{ color: "hsl(240 5% 45%)", fontSize: "12px", lineHeight: "1.7", margin: 0 }}>
          افتح البوت{" "}
          <span style={{ color: "hsl(262 83% 65%)", fontWeight: 600 }}>@RequiemAgentBot</span>
          {" "}في تلغرام ثم اضغط{" "}
          <span style={{ color: "hsl(262 83% 65%)", fontWeight: 600 }}>Launch</span>
        </p>
      </div>

      <style>{`
        @keyframes ra-float {
          0%, 100% { transform: translateY(0px); }
          50% { transform: translateY(-5px); }
        }
      `}</style>
    </div>
  );
}

// ── AuthGuard ─────────────────────────────────────────────────────────────────
export default function AuthGuard({ children }: { children: React.ReactNode }) {
  const { user, isLoading, isTelegram } = useAuth();

  if (isLoading) return <LoadingScreen />;
  if (!user)     return <AccessDeniedScreen inTelegram={isTelegram} />;

  return <>{children}</>;
}
