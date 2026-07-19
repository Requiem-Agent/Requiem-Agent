import { useRequireTelegram } from "@/hooks/use-auth";

export default function AuthGuard({ children }: { children: React.ReactNode }) {
  const { isReady, isLoading, isTelegram } = useRequireTelegram();

  if (isLoading) {
    return (
      <div style={{
        display: 'flex', height: '100vh', width: '100%',
        alignItems: 'center', justifyContent: 'center',
        background: '#0a0c10', color: '#e0e0e0',
        flexDirection: 'column', gap: '16px',
        fontFamily: "'Segoe UI', 'Cairo', 'Noto Sans Arabic', sans-serif",
      }}>
        <div style={{
          width: '28px', height: '28px', borderRadius: '50%',
          border: '2px solid #a855f7', borderTopColor: 'transparent',
          animation: 'spin 0.8s linear infinite',
        }} />
        <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
        <p style={{ color: '#888', fontSize: '14px', letterSpacing: '1px' }}>جاري التحميل...</p>
      </div>
    );
  }

  if (!isTelegram) {
    return (
      <div dir="rtl" style={{
        display: 'flex', height: '100vh', width: '100%',
        alignItems: 'center', justifyContent: 'center',
        background: '#0a0c10', color: '#e0e0e0',
        flexDirection: 'column', gap: '12px', padding: '32px',
        textAlign: 'center',
        fontFamily: "'Segoe UI', 'Cairo', 'Noto Sans Arabic', sans-serif",
      }}>
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          <path d="M9 12l2 2 4-4"/>
        </svg>
        
        <h1 style={{ fontSize: '22px', fontWeight: 700, margin: '8px 0 4px', color: '#ffffff' }}>
          وصول مقيد
        </h1>
        
        <p style={{ color: '#999', fontSize: '14px', lineHeight: '1.8', maxWidth: '360px', margin: '0' }}>
          لا يمكن الوصول إلى <bdi style={{unicodeBidi:'embed'}}>Requiem Agent</bdi> إلا من داخل
          تطبيق تلغرام عبر <bdi style={{unicodeBidi:'embed'}}>WebView</bdi>.
        </p>
        
        <p style={{ color: '#777', fontSize: '13px', lineHeight: '1.7', maxWidth: '320px', margin: '12px 0 0' }}>
          افتح البوت
          <bdi style={{unicodeBidi:'embed', color:'#a855f7', fontWeight:600}}> @RequiemAgentBot </bdi>
          في تلغرام ثم اضغط
          <bdi style={{unicodeBidi:'embed', color:'#a855f7', fontWeight:600}}> Launch </bdi>
          للبدء.
        </p>
        
        <div style={{
          marginTop: '20px', padding: '10px 24px',
          background: '#16162a', borderRadius: '10px',
          border: '1px solid #2a2a44', fontSize: '12px', color: '#666',
        }}>
          <bdi style={{unicodeBidi:'embed'}}>Telegram Mini Apps</bdi>
        </div>
      </div>
    );
  }

  if (!isReady) {
    return (
      <div dir="rtl" style={{
        display: 'flex', height: '100vh', width: '100%',
        alignItems: 'center', justifyContent: 'center',
        background: '#0a0c10', color: '#e0e0e0',
        flexDirection: 'column', gap: '12px',
        fontFamily: "'Segoe UI', 'Cairo', 'Noto Sans Arabic', sans-serif",
      }}>
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#ef4444" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="10"/>
          <line x1="15" y1="9" x2="9" y2="15"/>
          <line x1="9" y1="9" x2="15" y2="15"/>
        </svg>
        <p style={{ color: '#ef4444', fontSize: '15px', fontWeight: 500 }}>فشل التحقق</p>
        <p style={{ color: '#888', fontSize: '13px', margin: 0 }}>حاول مرة أخرى من تلغرام</p>
      </div>
    );
  }

  return <>{children}</>;
}
