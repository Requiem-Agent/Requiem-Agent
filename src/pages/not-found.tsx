import { Link } from "wouter";
import { AppLayout } from "@/components/layout";
import { Terminal, ArrowLeft } from "lucide-react";

export default function NotFound() {
  return (
    <AppLayout>
      <div className="flex flex-col items-center justify-center h-full gap-5 text-center px-6 animate-fade-in">
        <div className="h-14 w-14 rounded-2xl bg-primary/8 border border-primary/15 flex items-center justify-center">
          <Terminal className="h-7 w-7 text-primary/60" />
        </div>
        <div className="space-y-1.5">
          <h1 className="text-4xl font-bold font-mono text-primary/80">404</h1>
          <p className="text-sm font-medium text-muted-foreground">Page not found</p>
          <p className="text-xs text-muted-foreground/50">This route doesn't exist in Requiem Agent 1.</p>
        </div>
        <Link
          href="/"
          className="flex items-center gap-2 px-4 py-2.5 rounded-xl border border-primary/25 text-primary text-sm hover:bg-primary/8 transition-all"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to Agent
        </Link>
      </div>
    </AppLayout>
  );
}
