import { Link, useLocation } from "wouter";
import { Terminal, Bot as BotIcon, Settings, FolderOpen, Brain, CheckSquare, FolderClosed } from "lucide-react";
import { cn } from "@/lib/utils";
import { useState, useEffect } from "react";
import { Logo } from "@/components/logo";

const NAV_ITEMS = [
  { href: "/",           label: "الوكيل",    Icon: Terminal    },
  { href: "/workspaces", label: "المشاريع", Icon: FolderClosed },
  { href: "/files",      label: "الملفات",    Icon: FolderOpen  },
  { href: "/memory",     label: "الذاكرة",   Icon: Brain       },
  { href: "/bots",       label: "البوتات",     Icon: BotIcon     },
  { href: "/settings",   label: "الإعدادات", Icon: Settings    },
];

// ── Safe-area padding (iOS notch) ────────────────────────────────────────────
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
      {/* Top spacer */}
      <div
        className="shrink-0 w-full relative flex items-center justify-end px-3"
        style={{
          background: "hsl(var(--background))",
          height: topInset > 0
            ? `${topInset}px`
            : "max(env(safe-area-inset-top, 0px), 44px)",
        }}
      >
        {/* Version badge — top-right */}
        <span
          className="text-[10px] font-semibold tracking-wide px-2.5 py-1 rounded-full select-none"
          style={{
            background: "hsl(0 0% 0% / 0.06)",
            color:      "hsl(0 0% 35%)",
            border:     "1px solid hsl(0 0% 0% / 0.08)",
          }}
        >
          بوب كورن ستوديو
        </span>
      </div>

      {/* Page content */}
      <main className="flex-1 overflow-hidden min-h-0 relative">
        {children}
      </main>

      {/* Bottom navigation */}
      <nav
        className="shrink-0 flex items-center justify-around border-t select-none z-50"
        style={{
          background:     "hsl(var(--background))",
          height:         "60px",
          borderColor:    "hsl(0 0% 90%)",
          boxShadow:      "0 -1px 0 hsl(0 0% 90%), 0 -4px 20px hsl(0 0% 0% / 0.04)",
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
                "flex flex-col items-center justify-center flex-1 h-full gap-1 transition-all duration-200 relative",
                isActive ? "text-foreground" : "text-muted-foreground"
              )}
            >
              {/* Active indicator */}
              {isActive && (
                <span
                  className="absolute top-0 left-1/2 -translate-x-1/2 h-[2.5px] w-7 rounded-b-full"
                  style={{
                    background:  "#000000",
                    boxShadow:   "0 2px 8px hsl(0 0% 0% / 0.12)",
                  }}
                />
              )}

              {/* Icon */}
              <div
                className={cn(
                  "flex items-center justify-center h-8 w-8 rounded-xl transition-all duration-200",
                  isActive ? "bg-foreground/[0.08]" : "hover:bg-foreground/[0.04]"
                )}
              >
                <Icon
                  className={cn(
                    "transition-all duration-200",
                    isActive ? "h-[19px] w-[19px]" : "h-[17px] w-[17px]"
                  )}
                />
              </div>

              {/* Label */}
              <span
                className={cn(
                  "text-[10px] font-semibold tracking-wide leading-none transition-colors",
                  isActive ? "text-foreground" : "text-muted-foreground"
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
