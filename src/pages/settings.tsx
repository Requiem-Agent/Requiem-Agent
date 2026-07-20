import { useMemo, useState } from "react";
import { AppLayout } from "@/components/layout";
import { useUsageStats } from "@/hooks/use-system";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Loader2, Database, HardDrive, Terminal, Code2, Settings2,
  Palette, Bug, Search, Map, Shield, Cpu, Brain, Trash2, AlertTriangle,
  Bot, Zap, ChevronRight, User, Info, LogOut, Eye, EyeOff,
  Layers, Sparkles, CheckCircle2,
} from "lucide-react";
import { useRagStats, useClearMemory } from "@/hooks/use-memory";
import { useAuth } from "@/hooks/use-auth";
import { cn } from "@/lib/utils";

// ─── Ring Chart ───────────────────────────────────────────────────────────────
function RingChart({
  used, total, label, color,
}: {
  used: number; total: number; label: string; color: string;
}) {
  const pct = total > 0 ? Math.min(100, Math.max(0, (used / total) * 100)) : 0;
  const r = 34;
  const circ = 2 * Math.PI * r;
  const offset = circ - (pct / 100) * circ;

  return (
    <div className="flex flex-col items-center gap-2">
      <div className="relative h-24 w-24 flex items-center justify-center">
        <svg className="h-full w-full -rotate-90" viewBox="0 0 80 80">
          <circle stroke="hsl(var(--muted))" strokeWidth="6" cx="40" cy="40" r={r} fill="transparent" />
          <circle
            style={{ color, strokeDasharray: circ, strokeDashoffset: offset, transition: "stroke-dashoffset 1s ease" }}
            className="stroke-current"
            strokeWidth="6" strokeLinecap="round" cx="40" cy="40" r={r} fill="transparent"
          />
        </svg>
        <div className="absolute flex flex-col items-center">
          <span className="text-lg font-bold font-mono">{pct.toFixed(0)}%</span>
        </div>
      </div>
      <div className="text-center">
        <p className="text-xs font-medium">{label}</p>
        <p className="text-[10px] text-muted-foreground font-mono mt-0.5">{used.toLocaleString()} / {total.toLocaleString()}</p>
      </div>
    </div>
  );
}

// ─── Agent Mode Card ──────────────────────────────────────────────────────────
// NOTE: Model names are intentionally hidden — only "Requiem Agent 1" brand is shown
const AGENT_MODES = [
  { key: "orchestrator", Icon: Settings2, label: "Orchestrator", desc: "Main coordinator — distributes tasks across agents",  color: "text-primary   bg-primary/10   border-primary/20" },
  { key: "coder",        Icon: Code2,     label: "Coder",        desc: "Fast code generation and multi-file edits",           color: "text-cyan-400  bg-cyan-400/10  border-cyan-400/20" },
  { key: "planner",      Icon: Terminal,  label: "Planner",      desc: "Heavy reasoning and architectural planning",          color: "text-violet-400 bg-violet-400/10 border-violet-400/20" },
  { key: "debugger",     Icon: Bug,       label: "Debugger",     desc: "Large-scale debugging and root-cause analysis",       color: "text-rose-400  bg-rose-400/10  border-rose-400/20" },
  { key: "reviewer",     Icon: Cpu,       label: "Reviewer",     desc: "Dependency integrity and code quality",               color: "text-amber-400 bg-amber-400/10 border-amber-400/20" },
  { key: "researcher",   Icon: Search,    label: "Researcher",   desc: "Deep research and information synthesis",             color: "text-emerald-400 bg-emerald-400/10 border-emerald-400/20" },
  { key: "designer",     Icon: Palette,   label: "Designer",     desc: "UI/UX design and creative tasks",                    color: "text-pink-400  bg-pink-400/10  border-pink-400/20" },
  { key: "explorer",     Icon: Map,       label: "Explorer",     desc: "Codebase navigation and dependency mapping",          color: "text-indigo-400 bg-indigo-400/10 border-indigo-400/20" },
  { key: "security",     Icon: Shield,    label: "Security",     desc: "Vulnerability scanning and security analysis",        color: "text-orange-400 bg-orange-400/10 border-orange-400/20" },
];

export default function SettingsPage() {
  const { data: usage, isLoading: usageLoading } = useUsageStats();
  const { data: ragStats } = useRagStats();
  const clearMemory = useClearMemory();
  const { user, logout } = useAuth();
  const [clearConfirm, setClearConfirm] = useState(false);
  const [showUserId, setShowUserId] = useState(false);

  const readPct  = usage ? Math.min(100, ((usage.quotaReadUsed ?? 0) / (usage.readLimit ?? 500)) * 100) : 0;
  const writePct = usage ? Math.min(100, ((usage.quotaWriteUsed ?? 0) / (usage.writeLimit ?? 200)) * 100) : 0;

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 pt-4 pb-6 space-y-5 max-w-lg mx-auto w-full">

          {/* ── Header ── */}
          <div className="flex items-center gap-3 animate-slide-up">
            <div className="h-10 w-10 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-center">
              <Bot className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h1 className="text-base font-semibold tracking-tight">Settings</h1>
              <p className="text-xs text-muted-foreground">Requiem Agent 1 configuration</p>
            </div>
          </div>

          {/* ── Account ── */}
          <section className="animate-slide-up space-y-2" style={{ animationDelay: "40ms" }}>
            <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider px-1">Account</h2>
            <Card className="border-border/50 bg-card/40 overflow-hidden">
              <CardContent className="p-0">
                {/* User info */}
                <div className="flex items-center gap-3 px-4 py-3.5 border-b border-border/40">
                  <div className="h-9 w-9 rounded-full bg-primary/10 border border-primary/20 flex items-center justify-center shrink-0">
                    <User className="h-4 w-4 text-primary" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{user?.firstName || "Telegram User"}</p>
                    <p className="text-xs text-muted-foreground">
                      {user?.username ? `@${user.username}` : "Telegram Mini App"}
                    </p>
                  </div>
                  <Badge variant="outline" className="text-emerald-400 border-emerald-400/30 bg-emerald-400/5 text-[10px] font-mono shrink-0">
                    active
                  </Badge>
                </div>

                {/* Telegram ID */}
                {user?.telegramId ? (
                  <div className="flex items-center justify-between px-4 py-3 border-b border-border/40">
                    <span className="text-xs text-muted-foreground">Telegram ID</span>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-mono text-foreground/70">
                        {showUserId ? user.telegramId : "••••••••"}
                      </span>
                      <button onClick={() => setShowUserId(p => !p)} className="text-muted-foreground/50 hover:text-muted-foreground transition-colors">
                        {showUserId ? <EyeOff className="h-3 w-3" /> : <Eye className="h-3 w-3" />}
                      </button>
                    </div>
                  </div>
                ) : null}

                {/* Member since */}
                {user?.createdAt && (
                  <div className="flex items-center justify-between px-4 py-3 border-b border-border/40">
                    <span className="text-xs text-muted-foreground">Member since</span>
                    <span className="text-xs font-mono text-foreground/70">
                      {new Date(user.createdAt).toLocaleDateString("en", { month: "short", day: "numeric", year: "numeric" })}
                    </span>
                  </div>
                )}

                {/* Logout */}
                <button
                  onClick={logout}
                  className="flex items-center gap-2 w-full px-4 py-3 text-xs text-muted-foreground hover:text-destructive hover:bg-destructive/5 transition-all"
                >
                  <LogOut className="h-3.5 w-3.5" />
                  Sign out
                </button>
              </CardContent>
            </Card>
          </section>

          {/* ── Usage stats ── */}
          <section className="animate-slide-up space-y-2" style={{ animationDelay: "80ms" }}>
            <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider px-1">Usage</h2>
            <Card className="border-border/50 bg-card/40">
              <CardContent className="p-4">
                {usageLoading ? (
                  <div className="flex justify-center py-6">
                    <Loader2 className="h-5 w-5 animate-spin text-primary/60" />
                  </div>
                ) : (
                  <div className="flex justify-around">
                    <RingChart
                      used={usage?.quotaReadUsed ?? 0}
                      total={usage?.readLimit ?? 500}
                      label="Read quota"
                      color="hsl(var(--primary))"
                    />
                    <RingChart
                      used={usage?.quotaWriteUsed ?? 0}
                      total={usage?.writeLimit ?? 200}
                      label="Write quota"
                      color="hsl(var(--secondary))"
                    />
                  </div>
                )}
                {usage?.quotaResetAt && (
                  <p className="text-center text-[10px] text-muted-foreground/50 font-mono mt-3">
                    Resets {new Date(usage.quotaResetAt).toLocaleDateString("en", { month: "short", day: "numeric" })}
                  </p>
                )}
              </CardContent>
            </Card>
          </section>

          {/* ── Agent Modes ── */}
          <section className="animate-slide-up space-y-2" style={{ animationDelay: "120ms" }}>
            <div className="flex items-center gap-2 px-1">
              <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Agent Modes</h2>
              <div className="flex items-center gap-1 ml-auto">
                <Sparkles className="h-3 w-3 text-primary/60" />
                <span className="text-[10px] text-primary/60 font-mono">Requiem Agent 1</span>
              </div>
            </div>
            <div className="grid grid-cols-1 gap-2">
              {AGENT_MODES.map(({ key, Icon, label, desc, color }) => {
                const [textColor, bgColor, borderColor] = color.split(" ");
                return (
                  <div
                    key={key}
                    className={cn("flex items-center gap-3 px-3 py-2.5 rounded-xl border bg-card/20 transition-all", borderColor)}
                  >
                    <div className={cn("h-8 w-8 rounded-lg flex items-center justify-center border shrink-0", bgColor, borderColor)}>
                      <Icon className={cn("h-4 w-4", textColor)} />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="text-xs font-semibold">{label}</p>
                      <p className="text-[10px] text-muted-foreground/60 truncate">{desc}</p>
                    </div>
                    {/* No model name shown — only agent branding */}
                    <div className="flex items-center gap-1 shrink-0">
                      <div className="h-1.5 w-1.5 rounded-full bg-emerald-400/60" />
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* ── Memory / RAG ── */}
          {ragStats && (
            <section className="animate-slide-up space-y-2" style={{ animationDelay: "160ms" }}>
              <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider px-1">Memory</h2>
              <Card className="border-border/50 bg-card/40">
                <CardContent className="p-4 space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Brain className="h-4 w-4 text-violet-400" />
                      <span className="text-sm font-medium">RAG Memory</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-mono text-muted-foreground">{ragStats.total} entries</span>
                      {clearConfirm ? (
                        <div className="flex items-center gap-1.5">
                          <AlertTriangle className="h-3 w-3 text-amber-400" />
                          <button
                            onClick={() => { clearMemory.mutate(undefined); setClearConfirm(false); }}
                            disabled={clearMemory.isPending}
                            className="text-[10px] text-destructive font-mono px-2 py-0.5 rounded border border-destructive/40 hover:bg-destructive/10"
                          >
                            {clearMemory.isPending ? "..." : "yes"}
                          </button>
                          <button
                            onClick={() => setClearConfirm(false)}
                            className="text-[10px] text-muted-foreground font-mono px-2 py-0.5 rounded border border-border/50"
                          >
                            no
                          </button>
                        </div>
                      ) : (
                        <button
                          onClick={() => setClearConfirm(true)}
                          className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-destructive transition-colors font-mono px-2 py-0.5 rounded border border-border/50 hover:border-destructive/50"
                        >
                          <Trash2 className="h-2.5 w-2.5" /> clear
                        </button>
                      )}
                    </div>
                  </div>

                  {ragStats.total > 0 && (
                    <div className="grid grid-cols-4 gap-1.5">
                      {[
                        { label: "code",       color: "text-cyan-400",    bg: "bg-cyan-400/8"    },
                        { label: "fact",       color: "text-emerald-400", bg: "bg-emerald-400/8" },
                        { label: "pref",       color: "text-violet-400",  bg: "bg-violet-400/8"  },
                        { label: "context",    color: "text-amber-400",   bg: "bg-amber-400/8"   },
                      ].map(({ label, color, bg }) => (
                        <div key={label} className={cn("rounded-lg p-2.5 text-center border border-transparent", bg)}>
                          <p className={cn("text-base font-bold font-mono leading-none", color)}>
                            {(ragStats as any).by_type?.[label === "pref" ? "preference" : label] ?? 0}
                          </p>
                          <p className="text-[9px] text-muted-foreground/50 mt-1">{label}</p>
                        </div>
                      ))}
                    </div>
                  )}
                </CardContent>
              </Card>
            </section>
          )}

          {/* ── About ── */}
          <section className="animate-slide-up" style={{ animationDelay: "200ms" }}>
            <Card className="border-border/40 bg-card/20">
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <div className="h-10 w-10 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-center shrink-0">
                    <Bot className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <p className="text-sm font-semibold gradient-text">Requiem Agent 1</p>
                    <p className="text-[10px] text-muted-foreground/50">Powered by Requiem AI</p>
                  </div>
                </div>
                <div className="mt-3 pt-3 border-t border-border/40 flex items-center gap-3 text-[10px] text-muted-foreground/40 font-mono">
                  <span className="flex items-center gap-1"><CheckCircle2 className="h-2.5 w-2.5 text-emerald-400" /> Multi-model orchestration</span>
                  <span className="flex items-center gap-1"><CheckCircle2 className="h-2.5 w-2.5 text-emerald-400" /> RAG Memory</span>
                </div>
              </CardContent>
            </Card>
          </section>

          <div className="pb-4" />
        </div>
      </div>
    </AppLayout>
  );
}
