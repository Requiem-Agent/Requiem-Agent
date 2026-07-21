import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from '@/components/ui/toaster';
import { TooltipProvider } from '@/components/ui/tooltip';
import { Route, Switch, Router as WouterRouter } from 'wouter';
import { AuthProvider } from '@/hooks/use-auth';
import AuthGuard from '@/components/auth-guard';
import { SDKProvider, DisplayGate } from '@tma.js/sdk-react';
import { setBaseUrl } from '@workspace/api-client-react';

const apiUrl = import.meta.env.VITE_API_URL;
if (apiUrl) setBaseUrl(apiUrl);

// Pages
import WorkspacePage from '@/pages/workspace';
import FilesPage from '@/pages/files';
import MemoryPage from '@/pages/memory';
import TasksPage from '@/pages/tasks';
import BotsPage from '@/pages/bots';
import SettingsPage from '@/pages/settings';
import SandboxPage from '@/pages/sandbox';
import NotFound from '@/pages/not-found';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function Router() {
  return (
    <Switch>
      <Route path="/" component={WorkspacePage} />
      <Route path="/files" component={FilesPage} />
      <Route path="/memory" component={MemoryPage} />
      <Route path="/tasks" component={TasksPage} />
      <Route path="/bots" component={BotsPage} />
      <Route path="/settings" component={SettingsPage} />
      <Route path="/sandbox" component={SandboxPage} />
      <Route component={NotFound} />
    </Switch>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <SDKProvider options={{ cssVars: true, acceptCustomStyles: true, async: true }}>
        <DisplayGate
          error={({ error }) => (
            <div dir="rtl" style={{
              display:'flex',height:'100vh',width:'100%',alignItems:'center',justifyContent:'center',
              background:'#0a0c10',color:'#e0e0e0',flexDirection:'column',gap:'12px',padding:'32px',
              textAlign:'center',fontFamily:"'Segoe UI','Cairo','Noto Sans Arabic',sans-serif",
            }}>
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                <path d="M9 12l2 2 4-4"/>
              </svg>
              <h1 style={{fontSize:'22px',fontWeight:700,margin:'8px 0 4px',color:'#fff'}}>وصول مقيد</h1>
              <p style={{color:'#999',fontSize:'14px',lineHeight:'1.8',maxWidth:'360px',margin:'0'}}>
                لا يمكن الوصول إلى <bdi style={{unicodeBidi:'embed'}}>Requiem Agent</bdi> إلا من داخل
                تطبيق تلغرام عبر <bdi style={{unicodeBidi:'embed'}}>WebView</bdi>.
              </p>
              <p style={{color:'#777',fontSize:'13px',lineHeight:'1.7',maxWidth:'320px',margin:'12px 0 0'}}>
                افتح البوت
                <bdi style={{unicodeBidi:'embed',color:'#a855f7',fontWeight:600}}> @RequiemAgentBot </bdi>
                في تلغرام ثم اضغط
                <bdi style={{unicodeBidi:'embed',color:'#a855f7',fontWeight:600}}> Launch </bdi>
                للبدء.
              </p>
            </div>
          )}
          loading={
            <div style={{
              display:'flex',height:'100vh',width:'100%',alignItems:'center',justifyContent:'center',
              background:'#0a0c10',color:'#e0e0e0',flexDirection:'column',gap:'16px',
              fontFamily:"'Segoe UI','Cairo','Noto Sans Arabic',sans-serif",
            }}>
              <div style={{width:'28px',height:'28px',borderRadius:'50%',border:'2px solid #a855f7',borderTopColor:'transparent',animation:'spin 0.8s linear infinite'}}/>
              <style>{`@keyframes spin{to{transform:rotate(360deg)}}`}</style>
              <p style={{color:'#888',fontSize:'14px'}}>جاري التحميل...</p>
            </div>
          }
          initial={
            <div style={{
              display:'flex',height:'100vh',width:'100%',alignItems:'center',justifyContent:'center',
              background:'#0a0c10',color:'#e0e0e0',flexDirection:'column',gap:'24px',
              fontFamily:"'Segoe UI','Cairo','Noto Sans Arabic',sans-serif",
            }}>
              <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
              </svg>
              <div style={{width:'200px',height:'3px',background:'#1a1a2e',borderRadius:'2px',overflow:'hidden'}}>
                <div style={{height:'100%',background:'#a855f7',animation:'progress 2s ease-in-out infinite'}}/>
              </div>
              <style>{`@keyframes progress{0%{width:5%}50%{width:70%}100%{width:95%}}`}</style>
              <p style={{color:'#888',fontSize:'13px',margin:0}}>جاري الاتصال بتلغرام...</p>
            </div>
          }
        >
          <AuthProvider>
            <TooltipProvider>
              <WouterRouter base={import.meta.env.BASE_URL.replace(/\/$/, '')}>
                <AuthGuard>
                  <Router />
                </AuthGuard>
              </WouterRouter>
              <Toaster />
            </TooltipProvider>
          </AuthProvider>
        </DisplayGate>
      </SDKProvider>
    </QueryClientProvider>
  );
}

export default App;
