import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from '@/components/ui/toaster';
import { TooltipProvider } from '@/components/ui/tooltip';
import { Route, Switch, Router as WouterRouter } from 'wouter';
import { AuthProvider } from '@/hooks/use-auth';
import AuthGuard from '@/components/auth-guard';
import { setBaseUrl, setAuthTokenGetter } from '@workspace/api-client-react';

const apiUrl = import.meta.env.VITE_API_URL;
if (apiUrl) setBaseUrl(apiUrl);

// تمرير توكن الجلسة تلقائياً لكل طلبات api-client (sessions, bots, ...)
// وإلا تفشل الطلبات المحمية بـ 401 من التطبيق المصغر
setAuthTokenGetter(() => {
  try {
    return sessionStorage.getItem('rq_tok') || localStorage.getItem('requiem_token');
  } catch {
    return null;
  }
});

// Pages
import WorkspacePage from '@/pages/workspace';
import WorkspacesPage from '@/pages/workspaces';
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
      <Route path="/workspaces" component={WorkspacesPage} />
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
  const isDev = import.meta.env.DEV || window.location.search.includes('dev=1');

  if (isDev) {
    return (
      <QueryClientProvider client={queryClient}>
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
      </QueryClientProvider>
    );
  }

  return (
    <QueryClientProvider client={queryClient}>
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
    </QueryClientProvider>
  );
}


export default App;
