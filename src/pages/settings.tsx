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
  Layers, Sparkles, CheckCircle2, Key, Save, X, SlidersHorizontal,
  Bell, Lock, Thermometer, MessageSquare,
} from "lucide-react";
import { useRagStats, useClearMemory } from "@/hooks/use-memory";
import { useAuth } from "@/hooks/use-auth";
import { cn } from "@/lib/utils";
import {
  usePreferences,
  useUpdatePreferences,
  useApiKeys,
  useSaveApiKey,
  useDeleteApiKey,
  type UserPreferences,
} from "@/hooks/use-preferences";

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

// ─── Preferences Section ──────────────────────────────────────────────────────

function PreferencesSection() {
  const { data: prefs, isLoading } = usePreferences();
  const update = useUpdatePreferences();
  const [saved, setSaved] = useState(false);

  function toggle(field: keyof UserPreferences) {
    if (!prefs) return;
    const val = prefs[field];
    if (typeof val !== "boolean") return;
    update.mutate({ [field]: !val }, {
      onSuccess: () => { setSaved(true); setTimeout(() => setSaved(false), 1500); },
    });
  }

  function setField<K extends keyof UserPreferences>(field: K, value: UserPreferences[K]) {
    update.mutate({ [field]: value }, {
      onSuccess: () => { setSaved(true); setTimeout(() => setSaved(false), 1500); },
    });
  }

  if (isLoading) {
    return (
      <div className="flex justify-center py-6">
        <Loader2 className="h-5 w-5 animate-spin text-primary/60" />
      </div>
    );
  }

  if (!prefs) return null;

  return (
    <section className="animate-slide-up space-y-2" style={{ animationDelay: "100ms" }}>
      <div className="flex items-center gap-2 px-1">
        <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Preferences</h2>
        {saved && (
          <span className="flex items-center gap-1 text-[10px] text-emerald-400 font-mono ml-auto">
            <CheckCircle2 className="h-2.5 w-2.5" /> Saved
          </span>
        )}
      </div>

      {/* UI / UX */}
      <Card className="border-border/50 bg-card/40 overflow-hidden">
        <CardContent className="p-0">
          <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border/30 bg-muted/20">
            <Palette className="h-3 w-3 text-muted-foreground" />
            <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">Appearance</span>
          </div>

          {/* Theme */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-border/30">
            <span className="text-xs text-foreground/80">Theme</span>
            <div className="flex gap-1">
              {(["dark", "light", "system"] as const).map((t) => (
                <button
                  key={t}
                  onClick={() => setField("theme", t)}
                  className={cn(
                    "text-[10px] font-mono px-2 py-0.5 rounded border transition-all",
                    prefs.theme === t
                      ? "border-primary/60 bg-primary/10 text-primary"
                      : "border-border/40 text-muted-foreground hover:border-border"
                  )}
                >
                  {t}
                </button>
              ))}
            </div>
          </div>

          {/* Compact mode */}
          <ToggleRow
            label="Compact mode"
            value={prefs.compact_mode}
            onToggle={() => toggle("compact_mode")}
          />
          {/* Show timestamps */}
          <ToggleRow
            label="Show timestamps"
            value={prefs.show_timestamps}
            onToggle={() => toggle("show_timestamps")}
            last
          />
        </CardContent>
      </Card>

      {/* Agent behaviour */}
      <Card className="border-border/50 bg-card/40 overflow-hidden">
        <CardContent className="p-0">
          <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border/30 bg-muted/20">
            <SlidersHorizontal className="h-3 w-3 text-muted-foreground" />
            <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">Agent</span>
          </div>

          {/* Default mode */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-border/30">
            <span className="text-xs text-foreground/80">Default mode</span>
            <div className="flex gap-1">
              {(["chat", "orchestrator", "code"] as const).map((m) => (
                <button
                  key={m}
                  onClick={() => setField("default_mode", m)}
                  className={cn(
                    "text-[10px] font-mono px-2 py-0.5 rounded border transition-all",
                    prefs.default_mode === m
                      ? "border-primary/60 bg-primary/10 text-primary"
                      : "border-border/40 text-muted-foreground hover:border-border"
                  )}
                >
                  {m}
                </button>
              ))}
            </div>
          </div>

          {/* Temperature */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-border/30">
            <div className="flex items-center gap-1.5">
              <Thermometer className="h-3 w-3 text-muted-foreground" />
              <span className="text-xs text-foreground/80">Temperature</span>
            </div>
            <div className="flex items-center gap-2">
              <input
                type="range"
                min={0} max={1} step={0.1}
                value={prefs.temperature}
                onChange={(e) => setField("temperature", parseFloat(e.target.value))}
                className="w-20 accent-primary"
              />
              <span className="text-[10px] font-mono text-muted-foreground w-6 text-right">
                {prefs.temperature.toFixed(1)}
              </span>
            </div>
          </div>

          {/* Stream responses */}
          <ToggleRow
            label="Stream responses"
            value={prefs.stream_responses}
            onToggle={() => toggle("stream_responses")}
          />
          {/* Show thinking */}
          <ToggleRow
            label="Show thinking"
            value={prefs.show_thinking}
            onToggle={() => toggle("show_thinking")}
            last
          />
        </CardContent>
      </Card>

      {/* Notifications */}
      <Card className="border-border/50 bg-card/40 overflow-hidden">
        <CardContent className="p-0">
          <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border/30 bg-muted/20">
            <Bell className="h-3 w-3 text-muted-foreground" />
            <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">Notifications</span>
          </div>
          <ToggleRow label="On task complete" value={prefs.notify_on_complete} onToggle={() => toggle("notify_on_complete")} />
          <ToggleRow label="On error" value={prefs.notify_on_error} onToggle={() => toggle("notify_on_error")} />
          <ToggleRow label="On mention" value={prefs.notify_on_mention} onToggle={() => toggle("notify_on_mention")} last />
        </CardContent>
      </Card>

      {/* Privacy */}
      <Card className="border-border/50 bg-card/40 overflow-hidden">
        <CardContent className="p-0">
          <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border/30 bg-muted/20">
            <Lock className="h-3 w-3 text-muted-foreground" />
            <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">Privacy</span>
          </div>
          <ToggleRow label="Save history" value={prefs.save_history} onToggle={() => toggle("save_history")} />
          <ToggleRow label="Share analytics" value={prefs.share_analytics} onToggle={() => toggle("share_analytics")} last />
        </CardContent>
      </Card>
    </section>
  );
}

// ─── Toggle Row ───────────────────────────────────────────────────────────────

function ToggleRow({
  label, value, onToggle, last = false,
}: {
  label: string;
  value: boolean;
  onToggle: () => void;
  last?: boolean;
}) {
  return (
    <div className={cn("flex items-center justify-between px-4 py-3", !last && "border-b border-border/30")}>
      <span className="text-xs text-foreground/80">{label}</span>
      <button
        onClick={onToggle}
        className={cn(
          "relative h-5 w-9 rounded-full border transition-all duration-200",
          value
            ? "bg-primary/80 border-primary/60"
            : "bg-muted border-border/50"
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 h-4 w-4 rounded-full bg-white shadow transition-all duration-200",
            value ? "left-4" : "left-0.5"
          )}
        />
      </button>
    </div>
  );
}

// ─── API Keys Section ─────────────────────────────────────────────────────────

const PROVIDERS = [
  { id: "anthropic", label: "Anthropic", color: "text-orange-400" },
  { id: "openai",    label: "OpenAI",    color: "text-emerald-400" },
  { id: "gemini",    label: "Gemini",    color: "text-blue-400" },
  { id: "mistral",   label: "Mistral",   color: "text-violet-400" },
  { id: "groq",      label: "Groq",      color: "text-cyan-400" },
];

function ApiKeysSection() {
  const { data: keys, isLoading } = useApiKeys();
  const saveKey = useSaveApiKey();
  const deleteKey = useDeleteApiKey();
  const [adding, setAdding] = useState(false);
  const [provider, setProvider] = useState("anthropic");
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

  function handleSave() {
    if (!apiKey.trim()) return;
    saveKey.mutate({ provider, api_key: apiKey.trim() }, {
      onSuccess: () => { setAdding(false); setApiKey(""); },
    });
  }

  return (
    <section className="animate-slide-up space-y-2" style={{ animationDelay: "140ms" }}>
      <div className="flex items-center gap-2 px-1">
        <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">API Keys</h2>
        <button
          onClick={() => setAdding(true)}
          className="ml-auto flex items-center gap-1 text-[10px] text-primary/70 hover:text-primary font-mono px-2 py-0.5 rounded border border-primary/20 hover:border-primary/40 transition-all"
        >
          <Key className="h-2.5 w-2.5" /> Add key
        </button>
      </div>

      <Card className="border-border/50 bg-card/40 overflow-hidden">
        <CardContent className="p-0">
          {isLoading ? (
            <div className="flex justify-center py-4">
              <Loader2 className="h-4 w-4 animate-spin text-primary/60" />
            </div>
          ) : keys && keys.length > 0 ? (
            keys.map((k, i) => {
              const providerInfo = PROVIDERS.find((p) => p.id === k.provider);
              return (
                <div
                  key={k.id}
                  className={cn("flex items-center gap-3 px-4 py-3", i < keys.length - 1 && "border-b border-border/30")}
                >
                  <div className="flex-1 min-w-0">
                    <p className={cn("text-xs font-semibold", providerInfo?.color ?? "text-foreground")}>
                      {providerInfo?.label ?? k.provider}
                    </p>
                    <p className="text-[10px] font-mono text-muted-foreground">{k.key_hint}</p>
                  </div>
                  {deleteConfirm === k.id ? (
                    <div className="flex items-center gap-1.5">
                      <button
                        onClick={() => { deleteKey.mutate(k.id); setDeleteConfirm(null); }}
                        className="text-[10px] text-destructive font-mono px-2 py-0.5 rounded border border-destructive/40 hover:bg-destructive/10"
                      >
                        {deleteKey.isPending ? "..." : "yes"}
                      </button>
                      <button
                        onClick={() => setDeleteConfirm(null)}
                        className="text-[10px] text-muted-foreground font-mono px-2 py-0.5 rounded border border-border/50"
                      >
                        no
                      </button>
                    </div>
                  ) : (
                    <button
                      onClick={() => setDeleteConfirm(k.id)}
                      className="text-muted-foreground/40 hover:text-destructive transition-colors"
                    >
                      <X className="h-3.5 w-3.5" />
                    </button>
                  )}
                </div>
              );
            })
          ) : (
            <div className="flex flex-col items-center gap-1.5 py-5 text-center">
              <Key className="h-5 w-5 text-muted-foreground/30" />
              <p className="text-[10px] text-muted-foreground/50">No API keys saved yet</p>
            </div>
          )}

          {/* Add key form */}
          {adding && (
            <div className="border-t border-border/40 p-4 space-y-3 bg-muted/10">
              <div className="flex gap-1 flex-wrap">
                {PROVIDERS.map((p) => (
                  <button
                    key={p.id}
                    onClick={() => setProvider(p.id)}
                    className={cn(
                      "text-[10px] font-mono px-2 py-0.5 rounded border transition-all",
                      provider === p.id
                        ? "border-primary/60 bg-primary/10 text-primary"
                        : "border-border/40 text-muted-foreground hover:border-border"
                    )}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
              <div className="relative">
                <input
                  type={showKey ? "text" : "password"}
                  placeholder={`${provider} API key`}
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  className="w-full text-xs font-mono bg-background/50 border border-border/50 rounded-lg px-3 py-2 pr-8 focus:outline-none focus:border-primary/50 placeholder:text-muted-foreground/40"
                />
                <button
                  onClick={() => setShowKey((p) => !p)}
                  className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/40 hover:text-muted-foreground"
                >
                  {showKey ? <EyeOff className="h-3 w-3" /> : <Eye className="h-3 w-3" />}
                </button>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={handleSave}
                  disabled={!apiKey.trim() || saveKey.isPending}
                  className="flex items-center gap-1.5 text-[10px] font-mono px-3 py-1.5 rounded-lg bg-primary/80 text-primary-foreground hover:bg-primary disabled:opacity-50 transition-all"
                >
                  {saveKey.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />}
                  Save encrypted
                </button>
                <button
                  onClick={() => { setAdding(false); setApiKey(""); }}
                  className="text-[10px] font-mono px-3 py-1.5 rounded-lg border border-border/50 text-muted-foreground hover:border-border transition-all"
                >
                  Cancel
                </button>
              </div>
              {saveKey.isError && (
                <p className="text-[10px] text-destructive font-mono">{saveKey.error?.message}</p>
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}

export default function SettingsPage() {
  const { data: usage, isLoading: usageLoading } = useUsageStats();
  const { data: ragStats } = useRagStats();
  const clearMemory = useClearMemory();
  const { user, logout } = useAuth();
  const [clearConfirm, setClearConfirm] = useState(false);
  const [showUserId, setShowUserId] = useState(false);

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

          {/* ── S6-03: Preferences (GET/PUT /api/preferences) ── */}
          <PreferencesSection />

          {/* ── S6-02: API Keys ── */}
          <ApiKeysSection />

          {/* ── Usage stats ── */}
          <section className="animate-slide-up space-y-2" style={{ animationDelay: "180ms" }}>
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

          {/* ── Memory / RAG ── */}
          {ragStats && (
            <section className="animate-slide-up space-y-2" style={{ animationDelay: "220ms" }}>
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
                        { label: "code",    color: "text-cyan-400",    bg: "bg-cyan-400/8"    },
                        { label: "fact",    color: "text-emerald-400", bg: "bg-emerald-400/8" },
                        { label: "pref",    color: "text-violet-400",  bg: "bg-violet-400/8"  },
                        { label: "context", color: "text-amber-400",   bg: "bg-amber-400/8"   },
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
          <section className="animate-slide-up" style={{ animationDelay: "260ms" }}>
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