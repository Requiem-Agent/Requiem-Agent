import { useState, useEffect, ReactNode } from 'react';

interface InitResult {
  ready: boolean;
  initData: string;
  error: string | null;
}

/**
 * Reliable Telegram WebView detection.
 * 
 * The official telegram-web-app.js script creates window.Telegram.WebApp
 * in ALL environments (Telegram AND browser). In browsers, it creates
 * a mock with platform="unknown" and empty version.
 * 
 * Real Telegram WebView detection:
 * - window.Telegram?.WebApp?.platform !== 'unknown'
 * - window.Telegram?.WebApp?.version !== ''
 * - window.TelegramGameProxy !== undefined (Desktop)
 * 
 * Additionally, check these native Telegram bridges:
 * - window.Telegram?.WebView?.receiveEvent (iOS/Android)
 * - window.TelegramGameProxy?.receiveEvent (Desktop)
 */
function isRealTelegram(): boolean {
  const wa = (window as any).Telegram?.WebApp;
  if (!wa) return false;
  
  // Real Telegram has a real platform name (not "unknown")
  if (wa.platform && wa.platform !== 'unknown') return true;
  
  // Real Telegram has a Bot API version string
  if (wa.version) return true;
  
  // Native bridges (Desktop)
  if ((window as any).TelegramGameProxy) return true;
  if ((window as any).Telegram?.WebView?.receiveEvent) return true;
  
  return false;
}

export function TelegramInit({ children }: { children: ReactNode }) {
  const [result, setResult] = useState<InitResult>({
    ready: false, initData: '', error: null,
  });

  useEffect(() => {
    let attempts = 0;
    const maxAttempts = 15; // 15 * 300ms = 4.5 seconds max
    const interval = 300;

    const check = setInterval(() => {
      attempts++;

      if (isRealTelegram()) {
        clearInterval(check);
        const wa = (window as any).Telegram.WebApp;
        wa.ready();
        try { wa.expand(); } catch {}
        
        setResult({
          ready: true,
          initData: wa.initData || wa.initDataUnsafe?.query_id ? wa.initData : '',
          error: null,
        });
        return;
      }

      if (attempts >= maxAttempts) {
        clearInterval(check);
        setResult({
          ready: false,
          initData: '',
          error: 'Telegram WebView not detected',
        });
      }
    }, interval);

    return () => clearInterval(check);
  }, []);

  // Loading screen with progress bar
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
        <div style={{
          width: '200px', height: '3px', background: '#1a1a2e',
          borderRadius: '2px', overflow: 'hidden',
        }}>
          <div style={{
            height: '100%', background: '#a855f7',
            animation: 'progress 2s ease-in-out infinite',
          }} />
        </div>
        <style>{`@keyframes progress {
          0% { width: 5%; } 50% { width: 70%; } 100% { width: 95%; }
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

  // Success
  return <>{children}</>;
}
