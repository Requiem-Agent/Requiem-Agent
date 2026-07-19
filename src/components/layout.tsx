import { Link, useLocation } from "wouter";
import { Terminal, Bot as BotIcon, Settings } from "lucide-react";
import { useAuth } from "@/hooks/use-auth";

export function AppLayout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const { user } = useAuth();

  // If not authed, don't render nav
  if (!user) {
    return (
      <div className="flex flex-col h-[100dvh] bg-background text-foreground overflow-hidden">
        <main className="flex-1 overflow-hidden relative">
          {children}
        </main>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-[100dvh] bg-background text-foreground overflow-hidden">
      <main className="flex-1 overflow-hidden relative">
        {children}
      </main>
      <nav className="flex items-center justify-around border-t border-border bg-[#0a0a0c] h-14 shrink-0 px-2 select-none z-50 relative">
        <Link href="/" className={`flex flex-col items-center justify-center w-full h-full rounded-md transition-colors ${location === '/' ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:text-foreground hover:bg-white/5'}`}>
          <Terminal className="h-5 w-5" />
          <span className="text-[10px] mt-1 font-medium tracking-wide">Workspace</span>
        </Link>
        <Link href="/bots" className={`flex flex-col items-center justify-center w-full h-full rounded-md transition-colors ${location === '/bots' ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:text-foreground hover:bg-white/5'}`}>
          <BotIcon className="h-5 w-5" />
          <span className="text-[10px] mt-1 font-medium tracking-wide">Bots</span>
        </Link>
        <Link href="/settings" className={`flex flex-col items-center justify-center w-full h-full rounded-md transition-colors ${location === '/settings' ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:text-foreground hover:bg-white/5'}`}>
          <Settings className="h-5 w-5" />
          <span className="text-[10px] mt-1 font-medium tracking-wide">Settings</span>
        </Link>
      </nav>
    </div>
  );
}
