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

// ─── Safe Area Hook ────────────────────────────────────────────────────────────
// Telegram WebView injects its own header bar (≈44-56px) on top.
// We rely on CSS env() + a fixed constant for older Telegram clients.
const TG_EXTRA_PAD = 0; // px — extra padding on top of env(safe-area-inset-top)

export function AppLayout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const { user } = useAuth();

  if (!user) {
    return (
      <div
        className="flex flex-col bg-background text-foreground overflow-hidden"
        style={{
          height: "100dvh",
          paddingTop: `calc(env(safe-area-inset-top, 0px) + ${TG_EXTRA_PAD}px)`,
        }}
      >
        <main className="flex-1 overflow-hidden min-h-0 relative">{children}</main>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col bg-background text-foreground overflow-hidden"
      style={{
        height: "100dvh",
        /* Telegram header sits on top — push content below it */
        paddingTop: `calc(env(safe-area-inset-top, 0px) + ${TG_EXTRA_PAD}px)`,
        /* Bottom home-indicator on iOS */
        paddingBottom: "env(safe-area-inset-bottom, 0px)",
      }}
    >
      {/* ── Page content ── */}
      <main className="flex-1 overflow-hidden min-h-0 relative">{children}</main>

      {/* ── Bottom navigation ── */}
      <nav
        className="shrink-0 flex items-center justify-around border-t border-border/60 select-none z-50 relative"
        style={{
          background: "hsl(var(--background))",
          height: "56px",
          boxShadow: "0 -1px 0 hsl(var(--border) / 0.6), 0 -4px 16px hsl(0 0% 0% / 0.2)",
        }}
      >
        {NAV_ITEMS.map(({ href, label, Icon }) => {
          const isActive = href === "/" ? location === "/" : location.startsWith(href);
          return (
            <Link
              key={href}
              href={href}
              className={cn(
                "flex flex-col items-center justify-center flex-1 h-full gap-0.5 transition-all duration-200 relative",
                isActive ? "text-primary" : "text-muted-foreground/70"
              )}
            >
              {/* Active indicator pill */}
              {isActive && (
                <span
                  className="absolute top-0 left-1/2 -translate-x-1/2 h-0.5 w-8 rounded-b-full"
                  style={{ background: "hsl(var(--primary))", boxShadow: "0 2px 8px hsl(var(--primary) / 0.5)" }}
                />
              )}

              {/* Icon container */}
              <div
                className={cn(
                  "flex items-center justify-center h-7 w-7 rounded-xl transition-all duration-200",
                  isActive
                    ? "bg-primary/15"
                    : "hover:bg-white/[0.04]"
                )}
              >
                <Icon
                  className={cn(
                    "transition-all duration-200",
                    isActive ? "h-[18px] w-[18px]" : "h-4 w-4"
                  )}
                />
              </div>

              {/* Label */}
              <span
                className={cn(
                  "text-[9.5px] font-medium tracking-wide leading-none transition-colors",
                  isActive ? "text-primary" : "text-muted-foreground/50"
                )}
              >
                {label}
              </span>
            </Link>
          );
        })}
      </nav>
    </div>
  );
}
