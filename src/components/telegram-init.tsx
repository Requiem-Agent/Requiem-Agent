import { useState, useEffect, ReactNode } from 'react';

interface InitResult {
  ready: boolean;
  webApp: any | null;
  initData: string;
  error: string | null;
}

/**
 * Shows a loading screen while waiting for Telegram WebView to initialize.
 * Polls for window.Telegram.WebApp up to 5 seconds.
 * Only renders children when Telegram is confirmed present.
 */
export function TelegramInit({ children }: { children: ReactNode }) {
  const [result, setResult] = useState<InitResult>({
    ready: false, webApp: null, initData: '', error: null,
  });

  useEffect(() => {
    let attempts = 0;
    const maxAttempts = 10;
    const interval = 500; // check every 500ms

    const check = setInterval(() => {
      attempts++;
      const wa = (window as any).Telegram?.WebApp;

      if (wa && wa.version) {
        clearInterval(check);
        wa.ready();
        wa.expand();
        setResult({
          ready: true,
          webApp: wa,
          initData: wa.initData || '',
          error: null,
        });
        return;
      }

      if (attempts >= maxAttempts) {
        clearInterval(check);
        setResult({
          ready: false,
          webApp: null,
          initData: '',
          error: 'Telegram WebView not detected',
        });
      }
    }, interval);

    return () => clearInterval(check);
  }, []);

  // Loading screen
  if (!result.ready && !result.error) {
    return (
      <div style={{
        display: 'flex', height: '100vh', width: '100%',
        alignItems: 'center', justifyContent: 'center',
        background: '#0a0c10', color: '#e0e0e0',
        flexDirection: 'column', gap: '24px', padding: '24px',
        fontFamily: "'Segoe UI', 'Cairo', 'Noto Sans Arabic', sans-serif",
      }}>
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
        </svg>
        <h1 style={{ fontSize: '18px', fontWeight: 600, color: '#fff', margin: 0 }}>
          Requiem Agent
        </h1>
        {/* Progress bar */}
        <div style={{
          width: '200px', height: '3px', background: '#1a1a2e',
          borderRadius: '2px', overflow: 'hidden', marginTop: '4px',
        }}>
          <div style={{
            height: '100%', background: '#a855f7',
            borderRadius: '2px',
            animation: 'progress 2s ease-in-out infinite',
          }} />
        </div>
        <style>{`@keyframes progress {
          0% { width: 5%; }
          50% { width: 70%; }
          100% { width: 95%; }
        }`}</style>
        <p style={{ color: '#888', fontSize: '13px', margin: 0 }}>
          جاري الاتصال بتلغرام...
        </p>
      </div>
    );
  }

  // Error screen - not in Telegram
  if (result.error) {
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
        <h1 style={{ fontSize: '22px', fontWeight: 700, margin: '8px 0 4px', color: '#fff' }}>
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

  // Success - pass initData to children via context and render
  return <>{children}</>;
}
