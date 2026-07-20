import { Link, useLocation } from "wouter";
import { Terminal, Bot as BotIcon, Settings, FolderOpen, Brain, CheckSquare } from "lucide-react";
import { useAuth } from "@/hooks/use-auth";
import { cn } from "@/lib/utils";

const NAV_ITEMS = [
  { href: "/",         label: "Agent",    Icon: Terminal    },
  { href: "/files",    label: "Files",    Icon: FolderOpen  },
  { href: "/memory",   label: "Memory",   Icon: Brain       },
  { href: "/bots",     label: "Bots",     Icon: BotIcon     },
  { href: "/tasks",    label: "Tasks",    Icon: CheckSquare },
  { href: "/settings", label: "Settings", Icon: Settings    },
];

export function AppLayout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const { user } = useAuth();

  if (!user) {
    return (
      <div className="flex flex-col h-[100dvh] bg-background text-foreground overflow-hidden">
        <main className="flex-1 overflow-hidden relative">{children}</main>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-[100dvh] bg-background text-foreground overflow-hidden">
      <main className="flex-1 overflow-hidden relative">{children}</main>

      {/* Bottom nav — compact for 6 items */}
      <nav className="flex items-center justify-around border-t border-border bg-[#0a0a0c] h-14 shrink-0 px-1 select-none z-50 relative">
        {NAV_ITEMS.map(({ href, label, Icon }) => {
          const isActive = href === "/" ? location === "/" : location.startsWith(href);
          return (
            <Link
              key={href}
              href={href}
              className={cn(
                "flex flex-col items-center justify-center flex-1 h-full rounded-md transition-all duration-200",
                isActive
                  ? "text-primary"
                  : "text-muted-foreground hover:text-foreground hover:bg-white/[0.03]"
              )}
            >
              <div className={cn(
                "relative flex items-center justify-center h-6 w-6 rounded-md transition-all duration-200",
                isActive && "bg-primary/15"
              )}>
                <Icon className={cn("h-4 w-4 transition-transform", isActive && "scale-110")} />
                {isActive && (
                  <span className="absolute -bottom-0.5 left-1/2 -translate-x-1/2 h-0.5 w-3 bg-primary rounded-full" />
                )}
              </div>
              <span className={cn(
                "text-[9px] mt-1 font-medium tracking-wide transition-colors",
                isActive ? "text-primary" : "text-muted-foreground/60"
              )}>
                {label}
              </span>
            </Link>
          );
        })}
      </nav>
    </div>
  );
}
