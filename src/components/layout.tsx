import { Link, useLocation } from "wouter";
import { Terminal, Bot as BotIcon, Settings, FolderOpen, Brain, CheckSquare, FolderClosed } from "lucide-react";
import { cn } from "@/lib/utils";
import { useState, useEffect } from "react";

const NAV_ITEMS = [
  { href: "/",           label: "Agent",    Icon: Terminal    },
  { href: "/workspaces", label: "Projects", Icon: FolderClosed },
  { href: "/files",      label: "Files",    Icon: FolderOpen  },
  { href: "/memory",     label: "Memory",   Icon: Brain       },
  { href: "/bots",       label: "Bots",     Icon: BotIcon     },
  { href: "/settings",   label: "Settings", Icon: Settings    },
];

// ── Safe-area padding (iOS notch) ────────────────────────────────────────────
// يُستخدم env(safe-area-inset-top) من CSS فقط — لا كشف لتلغرام

function useSafeArea() {
  const [topInset, setTopInset] = useState(0);

  useEffect(() => {
    const readInset = () => {
      const raw = getComputedStyle(document.documentElement)
        .getPropertyValue("--safe-area-inset-top").trim();
      setTopInset(raw ? (parseInt(raw) || 0) : 0);
    };
    readInset();
    window.addEventListener("resize", readInset);
    return () => window.removeEventListener("resize", readInset);
  }, []);

  return topInset;
}

// ── AppLayout ─────────────────────────────────────────────────────────────────
export function AppLayout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const topInset   = useSafeArea();

  return (
    <div
      className="flex flex-col bg-background text-foreground overflow-hidden"
      style={{ height: "100dvh" }}
    >
      {/* Top spacer: protects content from TG header buttons.
          Uses CSS env() as the baseline (applies before JS), then JS
          overrides with exact inset.
          When topInset > 0 (JS measured), use it directly.
          Otherwise rely on CSS env(safe-area-inset-top) via paddingTop. */}
      <div
        className="shrink-0 w-full relative flex items-center justify-end px-3"
        style={{
          background: "hsl(var(--background))",
          // If JS measured a value, use it; otherwise use CSS env() with 44px min
          height: topInset > 0
            ? `${topInset}px`
            : "max(env(safe-area-inset-top, 0px), 44px)",
        }}
      >
        {/* Version badge — top-right of safe area */}
        <span
          className="text-[9px] font-semibold tracking-wide px-2 py-0.5 rounded-full select-none"
          style={{
            background: "hsl(262 83% 62% / 0.15)",
            color:      "hsl(262 83% 75%)",
            border:     "1px solid hsl(262 83% 62% / 0.25)",
          }}
        >
          Requiem Agent 1.2
        </span>
      </div>

      {/* Page content */}
      <main className="flex-1 overflow-hidden min-h-0 relative">
        {children}
      </main>

      {/* Bottom navigation */}
      <nav
        className="shrink-0 flex items-center justify-around border-t border-border/60 select-none z-50"
        style={{
          background:     "hsl(var(--background))",
          height:         "56px",
          boxShadow:      "0 -1px 0 hsl(var(--border) / 0.6), 0 -4px 16px hsl(0 0% 0% / 0.25)",
          // iOS/Android home-indicator safe area
          paddingBottom:  "max(env(safe-area-inset-bottom, 0px), 4px)",
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
              {/* Active pill at top */}
              {isActive && (
                <span
                  className="absolute top-0 left-1/2 -translate-x-1/2 h-0.5 w-8 rounded-b-full"
                  style={{
                    background:  "hsl(var(--primary))",
                    boxShadow:   "0 2px 8px hsl(var(--primary) / 0.5)",
                  }}
                />
              )}

              {/* Icon */}
              <div
                className={cn(
                  "flex items-center justify-center h-7 w-7 rounded-xl transition-all duration-200",
                  isActive ? "bg-primary/15" : "hover:bg-white/[0.04]"
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
