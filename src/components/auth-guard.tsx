import { useAuth } from "@/hooks/use-auth";
import { Loader2 } from "lucide-react";

export default function AuthGuard({ children }: { children: React.ReactNode }) {
  const { user, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <p className="text-sm text-muted-foreground font-mono">INITIALIZING_SESSION...</p>
        </div>
      </div>
    );
  }

  if (!user) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4 text-center max-w-sm px-4">
          <div className="h-16 w-16 rounded-xl bg-card border border-border flex items-center justify-center mb-4">
            <div className="h-8 w-8 rounded-full bg-primary/20 flex items-center justify-center">
              <div className="h-4 w-4 bg-primary rounded-full shadow-[0_0_15px_rgba(124,58,237,0.5)]"></div>
            </div>
          </div>
          <h1 className="text-2xl font-bold tracking-tight">Requiem Agent</h1>
          <p className="text-sm text-muted-foreground mb-8">
            Access denied. Please launch this app directly from the Telegram bot.
          </p>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
