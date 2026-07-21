import { Link, useLocation } from "wouter";
import { Terminal, Bot as BotIcon, Settings, FolderOpen, Brain, CheckSquare } from "lucide-react";
import { cn } from "@/lib/utils";
import { useState, useEffect } from "react";

const NAV_ITEMS = [
  { href: "/",         label: "Agent",    Icon: Terminal    },
  { href: "/files",    label: "Files",    Icon: FolderOpen  },
  { href: "/memory",   label: "Memory",   Icon: Brain       },
  { href: "/bots",     label: "Bots",     Icon: BotIcon     },
  { href: "/tasks",    label: "Tasks",    Icon: CheckSquare },
  { href: "/settings", label: "Settings", Icon: Settings    },
];

// ── Telegram safe-area hook ───────────────────────────────────────────────────
// Telegram WebApp safeAreaInsets (Bot API 7.7+): the header buttons area height in px.
// contentSafeAreaInsets: additional content padding (usually 0).
// CSS var --tg-safe-area-inset-top is set by @tma.js/sdk-react with cssVars:true, but
// may lag behind initialization, so we also read directly from the TG WebApp object.
//
// MINIMUM TOP INSET: Telegram Mini Apps always show a header bar with Back/Close buttons
// (roughly 44-56px depending on platform). We guarantee a minimum of 8px so content
// never bleeds under the TG header even if safeAreaInsets reports 0 during init.
const MIN_TOP_INSET = 8;

function useTelegramSafeArea() {
  const [topInset, setTopInset] = useState(() => {
    // Try to read synchronously on first render
    const tg = (window as any).Telegram?.WebApp;
    // Also check CSS variable set by @tma.js/sdk
    const cssVar = parseInt(getComputedStyle(document.documentElement)
      .getPropertyValue("--tg-safe-area-inset-top") || "0") || 0;
    const safeTop    = tg?.safeAreaInsets?.top         ?? cssVar;
    const contentTop = tg?.contentSafeAreaInsets?.top  ?? 0;
    const isTgWebApp = !!(tg?.initData);
    // In Telegram: always add minimum inset. Outside: 0.
    return isTgWebApp ? Math.max(safeTop + contentTop, MIN_TOP_INSET) : 0;
  });

  useEffect(() => {
    const tg = (window as any).Telegram?.WebApp;
    if (!tg) return;

    // Expand to full height so content fills the WebView
    tg.expand?.();
    // On newer TG clients, request full-screen mode
    tg.requestFullscreen?.();
    // Disable vertical swipe to dismiss (prevents accidental close)
    tg.disableVerticalSwipes?.();

    const readInsets = () => {
      const cssVar = parseInt(getComputedStyle(document.documentElement)
        .getPropertyValue("--tg-safe-area-inset-top") || "0") || 0;
      const safeTop    = tg.safeAreaInsets?.top         ?? cssVar;
      const contentTop = tg.contentSafeAreaInsets?.top  ?? 0;
      const isTgWebApp = !!(tg.initData);
      setTopInset(isTgWebApp ? Math.max(safeTop + contentTop, MIN_TOP_INSET) : 0);
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

  // Check if we're inside Telegram WebView (for additional styling)
  const isTelegram = !!(typeof window !== "undefined" && (window as any).Telegram?.WebApp?.initData);

  return (
    <div
      className="flex flex-col bg-background text-foreground overflow-hidden"
      style={{ height: "100dvh" }}
    >
      {/* Telegram header spacer — always present in TG to avoid header overlap */}
      {(topInset > 0 || isTelegram) && (
        <div
          className="shrink-0 w-full"
          style={{
            height: `${Math.max(topInset, isTelegram ? MIN_TOP_INSET : 0)}px`,
            background: "hsl(var(--background))",
          }}
          aria-hidden="true"
        />
      )}

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
