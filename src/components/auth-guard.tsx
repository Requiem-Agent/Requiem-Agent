import { useRequireTelegram } from "@/hooks/use-auth";

export default function AuthGuard({ children }: { children: React.ReactNode }) {
  const { isReady, isLoading, isTelegram } = useRequireTelegram();

  if (isLoading) {
    return (
      <div style={{
        display: 'flex', height: '100vh', width: '100%',
        alignItems: 'center', justifyContent: 'center',
        background: '#0a0c10', color: '#fff',
        flexDirection: 'column', gap: '12px', fontFamily: 'monospace',
      }}>
        <div style={{
          width: '32px', height: '32px', borderRadius: '50%',
          border: '2px solid #a855f7', borderTopColor: 'transparent',
          animation: 'spin 0.8s linear infinite',
        }} />
        <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
        <p style={{ color: '#888', fontSize: '13px' }}>INITIALIZING...</p>
      </div>
    );
  }

  if (!isTelegram) {
    return (
      <div style={{
        display: 'flex', height: '100vh', width: '100%',
        alignItems: 'center', justifyContent: 'center',
        background: '#0a0c10', color: '#fff',
        flexDirection: 'column', gap: '16px', padding: '24px', textAlign: 'center',
        fontFamily: 'monospace',
      }}>
        <div style={{ fontSize: '48px', marginBottom: '8px' }}>🔒</div>
        <h1 style={{ fontSize: '20px', margin: 0, fontWeight: 600 }}>Telegram Only</h1>
        <p style={{ color: '#888', maxWidth: '320px', lineHeight: '1.5', margin: 0, fontSize: '14px' }}>
          Requiem Agent can only be accessed through Telegram WebView.
        </p>
        <p style={{ color: '#666', fontSize: '13px', maxWidth: '280px', margin: '8px 0 0' }}>
          Open <b style={{ color: '#a855f7' }}>@RequiemAgentBot</b> in Telegram and press <b>Launch</b>.
        </p>
        <div style={{
          marginTop: '16px', padding: '10px 20px', background: '#1a1a2e',
          borderRadius: '8px', border: '1px solid #2a2a3e', fontSize: '12px', color: '#666',
        }}>
          Telegram Mini Apps required
        </div>
      </div>
    );
  }

  if (!isReady) {
    return (
      <div style={{
        display: 'flex', height: '100vh', width: '100%',
        alignItems: 'center', justifyContent: 'center',
        background: '#0a0c10', color: '#ff4444', fontFamily: 'monospace',
        flexDirection: 'column', gap: '12px',
      }}>
        <div style={{ fontSize: '32px' }}>✕</div>
        <p>Authentication failed</p>
        <p style={{ color: '#666', fontSize: '12px' }}>Please try again from Telegram</p>
      </div>
    );
  }

  return <>{children}</>;
}
