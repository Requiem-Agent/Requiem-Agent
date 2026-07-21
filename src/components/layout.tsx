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

// ── Telegram safe-area detection ─────────────────────────────────────────────
// Telegram Mini Apps have a header bar (~44-56px) with Close/Back buttons.
// We MUST pad the top of our UI so content doesn't overlap with those buttons.
//
// Strategy (in priority order):
//   1. tg.safeAreaInsets.top + tg.contentSafeAreaInsets.top (Bot API 7.7+)
//   2. CSS var --tg-safe-area-inset-top (set by @tma.js/sdk-react)
//   3. env(safe-area-inset-top) from CSS (iOS notch)
//   4. Hard minimum: 44px when inside Telegram (standard header height)
//
// We use CSS for the actual padding so it applies immediately before JS hydrates,
// preventing the layout from jumping on first render.

function useTelegramSafeArea() {
  const [topInset, setTopInset] = useState(0);

  useEffect(() => {
    const tg = (window as any).Telegram?.WebApp;
    const isTg = !!(tg?.initData || tg?.initDataUnsafe);

    if (!isTg) return; // Not in Telegram — no inset needed

    // Expand to full height
    tg.expand?.();
    // Request fullscreen on newer TG versions
    tg.requestFullscreen?.();
    // Prevent accidental close via swipe
    tg.disableVerticalSwipes?.();

    const readInsets = () => {
      // Method 1: tg.safeAreaInsets (most accurate, Bot API 7.7+)
      const safeTop    = (tg.safeAreaInsets?.top        ?? 0) as number;
      const contentTop = (tg.contentSafeAreaInsets?.top ?? 0) as number;

      // Method 2: CSS variable from @tma.js/sdk
      const cssVarRaw = getComputedStyle(document.documentElement)
        .getPropertyValue("--tg-safe-area-inset-top").trim();
      const cssVar = cssVarRaw ? parseInt(cssVarRaw) || 0 : 0;

      // Take the best reading: safeTop+contentTop if non-zero, else cssVar
      // Then ensure minimum 44px (standard TG header height)
      const raw = safeTop + contentTop > 0 ? safeTop + contentTop : cssVar;
      setTopInset(Math.max(raw, 44)); // Always at least 44px in Telegram
    };

    readInsets();

    tg.onEvent?.("safeAreaChanged",        readInsets);
    tg.onEvent?.("contentSafeAreaChanged", readInsets);
    tg.onEvent?.("viewportChanged",        readInsets);

    return () => {
      tg.offEvent?.("safeAreaChanged",        readInsets);
      tg.offEvent?.("contentSafeAreaChanged", readInsets);
      tg.offEvent?.("viewportChanged",        readInsets);
    };
  }, []);

  return topInset;
}

// ── AppLayout ─────────────────────────────────────────────────────────────────
export function AppLayout({ children }: { children: React.ReactNode }) {
  const [location] = useLocation();
  const topInset   = useTelegramSafeArea();

  return (
    <div
      className="flex flex-col bg-background text-foreground overflow-hidden"
      style={{ height: "100dvh" }}
    >
      {/* Top spacer: protects content from TG header buttons.
          Uses CSS env() as the baseline (applies before JS), then JS
          overrides with exact inset once Telegram SDK reports it.
          When topInset > 0 (JS measured), use it directly.
          Otherwise rely on CSS env(safe-area-inset-top) via paddingTop. */}
      <div
        className="shrink-0 w-full"
        style={{
          background: "hsl(var(--background))",
          // If JS measured a value, use it; otherwise use CSS env() with 44px min
          height: topInset > 0
            ? `${topInset}px`
            : "max(env(safe-area-inset-top, 0px), 44px)",
        }}
        aria-hidden="true"
      />

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
