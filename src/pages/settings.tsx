import { useMemo } from "react";
import { AppLayout } from "@/components/layout";
import { useModels, useUsageStats, ROLE_MODEL_MAP } from "@/hooks/use-system";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Loader2, Database, HardDrive,
  Terminal, Code2, Settings2, Palette, Bug, Search, Map, Shield, Cpu,
  Brain, Trash2, AlertTriangle } from "lucide-react";
import { useRagStats, useClearMemory } from "@/hooks/use-memory";
import { useState } from "react";

// ─── Ring Chart ───────────────────────────────────────────────────────────────
function RingChart({
  used, total, label, color, format = "number",
}: {
  used: number; total: number; label: string; color: string; format?: "number" | "bytes";
}) {
  const percentage = total > 0 ? Math.min(100, Math.max(0, (used / total) * 100)) : 0;
  const radius = 40;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (percentage / 100) * circumference;
  const displayUsed = format === "bytes" ? `${(used / 1024 / 1024).toFixed(1)}MB` : used.toLocaleString();
  const displayTotal = format === "bytes" ? `${(total / 1024 / 1024).toFixed(1)}MB` : total.toLocaleString();

  return (
    <div className="flex flex-col items-center justify-center">
      <div className="relative h-32 w-32 flex items-center justify-center">
        <svg className="h-full w-full -rotate-90 transform" viewBox="0 0 100 100">
          <circle className="text-muted stroke-current" strokeWidth="8" cx="50" cy="50" r={radius} fill="transparent" />
          <circle
            className="stroke-current transition-all duration-1000 ease-out"
            style={{ color, strokeDasharray: circumference, strokeDashoffset }}
            strokeWidth="8" strokeLinecap="round" cx="50" cy="50" r={radius} fill="transparent"
          />
        </svg>
        <div className="absolute flex flex-col items-center justify-center text-center">
          <span className="text-xl font-bold font-mono tracking-tighter">{percentage.toFixed(0)}%</span>
        </div>
      </div>
      <div className="mt-4 text-center">
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted-foreground font-mono mt-1">{displayUsed} / {displayTotal}</p>
      </div>
    </div>
  );
}

// ─── Agent Mode Config ────────────────────────────────────────────────────────
const AGENT_MODES = [
  {
    key: "orchestrator",
    Icon: Settings2,
    label: "Orchestrator",
    description: "Main coordinator — distributes tasks across models, manages context and vision.",
    model: "mimo-v2.5-free",
    modelLabel: "Mimo V2.5",
    color: "text-primary bg-primary/10 border-primary/20",
  },
  {
    key: "coder",
    Icon: Code2,
    label: "Coder",
    description: "Fast code generation, file edits, and multi-file parallel development.",
    model: "deepseek-v4-flash-free",
    modelLabel: "Deepseek V4 Flash",
    color: "text-cyan-400 bg-cyan-400/10 border-cyan-400/20",
  },
  {
    key: "planner",
    Icon: Terminal,
    label: "Planner",
    description: "Heavy reasoning, architectural planning, and logic analysis.",
    model: "hy3-free",
    modelLabel: "Hy3",
    color: "text-violet-400 bg-violet-400/10 border-violet-400/20",
  },
  {
    key: "debugger",
    Icon: Bug,
    label: "Debugger",
    description: "Large-scale testing, root-cause analysis, and multi-layer debugging.",
    model: "nemotron-3-ultra-free",
    modelLabel: "Nemotron Ultra",
    color: "text-destructive bg-destructive/10 border-destructive/20",
  },
  {
    key: "reviewer",
    Icon: Cpu,
    label: "Reviewer",
    description: "Dependency integrity, dead-code detection, and code cleanliness.",
    model: "north-mini-code-free",
    modelLabel: "North Mini Code",
    color: "text-emerald-400 bg-emerald-400/10 border-emerald-400/20",
  },
  {
    key: "designer",
    Icon: Palette,
    label: "Designer",
    description: "UI/UX direction, visual identity, and frontend architecture.",
    model: "mimo-v2.5-free",
    modelLabel: "Mimo V2.5",
    color: "text-pink-400 bg-pink-400/10 border-pink-400/20",
  },
  {
    key: "researcher",
    Icon: Search,
    label: "Researcher",
    description: "Web research, documentation analysis, and knowledge gathering.",
    model: "hy3-free",
    modelLabel: "Hy3",
    color: "text-amber-400 bg-amber-400/10 border-amber-400/20",
  },
  {
    key: "explorer",
    Icon: Map,
    label: "Explorer",
    description: "Parallel multi-file exploration and codebase mapping.",
    model: "big-pickle",
    modelLabel: "Big Pickle",
    color: "text-orange-400 bg-orange-400/10 border-orange-400/20",
  },
  {
    key: "security",
    Icon: Shield,
    label: "Security",
    description: "Vulnerability detection, secure coding, and pentest analysis.",
    model: "deepseek-v4-flash-free",
    modelLabel: "Deepseek V4 Flash",
    color: "text-red-400 bg-red-400/10 border-red-400/20",
  },
] as const;

const EFFORT_INFO = [
  {
    key: "lite",
    label: "Lite",
    description: "Quick planning, analysis, and simple single-step tasks.",
    color: "text-muted-foreground bg-muted border-border",
  },
  {
    key: "medium",
    label: "Medium",
    description: "Standard coding tasks, feature verification, format checks.",
    color: "text-cyan-400 bg-cyan-400/10 border-cyan-400/30",
  },
  {
    key: "high",
    label: "High",
    description: "Precise debugging, root-cause analysis, strict validation.",
    color: "text-amber-400 bg-amber-400/10 border-amber-400/30",
  },
  {
    key: "max",
    label: "Max",
    description: "Full project builds, parallel development, multi-model orchestration.",
    color: "text-destructive bg-destructive/10 border-destructive/30",
  },
] as const;

// ─── Main Page ────────────────────────────────────────────────────────────────
export default function SettingsPage() {
  const { data: modelsData, isLoading: modelsLoading } = useModels();
  const { data: ragStats, isLoading: ragLoading } = useRagStats();
  const clearMemory = useClearMemory();
  const [clearConfirm, setClearConfirm] = useState(false);
  const { data: usageData, isLoading: usageLoading } = useUsageStats();

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto p-4 md:p-8 space-y-10 pb-24">

        {/* Header */}
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground font-mono">/settings</h1>
          <p className="text-sm text-muted-foreground mt-1">Requiem Agent — pipeline overview and usage</p>
        </div>

        {/* ─── Agent Modes ─────────────────────────────────────────────────── */}
        <section>
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-widest font-mono mb-4">
            Agent Modes
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {AGENT_MODES.map(({ key, Icon, label, description, model, modelLabel, color }) => (
              <Card key={key} className="border-border/50 bg-[#0d0d0f] hover:border-border transition-colors">
                <CardHeader className="pb-2 pt-4 px-4">
                  <div className="flex items-center gap-2">
                    <div className={`h-8 w-8 rounded-md border flex items-center justify-center ${color}`}>
                      <Icon className="h-4 w-4" />
                    </div>
                    <div>
                      <CardTitle className="text-sm font-semibold">{label}</CardTitle>
                    </div>
                  </div>
                </CardHeader>
                <CardContent className="px-4 pb-4 space-y-2">
                  <p className="text-xs text-muted-foreground leading-relaxed">{description}</p>
                  <div className="flex items-center gap-1.5 pt-1">
                    <Cpu className="h-3 w-3 text-muted-foreground/60 shrink-0" />
                    <span className="text-[10px] font-mono text-muted-foreground/80 truncate">{modelLabel}</span>
                    <span className="text-[10px] font-mono text-muted-foreground/40">({model})</span>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>

        {/* ─── Effort Levels ───────────────────────────────────────────────── */}
        <section>
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-widest font-mono mb-4">
            Effort Levels
          </h2>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            {EFFORT_INFO.map(({ key, label, description, color }) => (
              <Card key={key} className="border-border/50 bg-[#0d0d0f]">
                <CardContent className="p-4">
                  <Badge variant="outline" className={`capitalize font-mono text-[10px] mb-2 ${color}`}>
                    {label}
                  </Badge>
                  <p className="text-xs text-muted-foreground leading-relaxed">{description}</p>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>

        {/* ─── Active Models (6 Free from OpenCode Zen) ────────────────────── */}
        <section>
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-widest font-mono mb-4">
            Active Models — OpenCode Zen (Free Tier)
          </h2>
          {modelsLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground text-sm">
              <Loader2 className="h-4 w-4 animate-spin" /> Loading...
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {modelsData?.models.map((model) => {
                const modeEntry = AGENT_MODES.find(m => m.key === model.assignedRole);
                const ModeIcon = modeEntry?.Icon ?? Cpu;
                const modeColor = modeEntry?.color ?? "text-muted-foreground bg-muted border-border";
                return (
                  <div key={model.id} className="flex items-center gap-3 p-3 rounded-lg border border-border/50 bg-[#0d0d0f]">
                    <div className={`h-8 w-8 rounded-md border flex items-center justify-center shrink-0 ${modeColor}`}>
                      <ModeIcon className="h-3.5 w-3.5" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <p className="text-sm font-medium truncate">{model.name}</p>
                      <p className="text-[10px] font-mono text-muted-foreground truncate">{model.id}</p>
                    </div>
                    <Badge variant="outline" className={`capitalize font-mono text-[10px] shrink-0 ${modeColor}`}>
                      {model.assignedRole}
                    </Badge>
                  </div>
                );
              })}
            </div>
          )}
        </section>

        {/* ─── Usage ───────────────────────────────────────────────────────── */}
        <section>
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-widest font-mono mb-4">
            Usage Quota
          </h2>
          {usageLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground text-sm">
              <Loader2 className="h-4 w-4 animate-spin" /> Loading usage...
            </div>
          ) : usageData ? (
            <Card className="border-border/50 bg-[#0d0d0f]">
              <CardContent className="p-6">
                <div className="grid grid-cols-2 md:grid-cols-4 gap-8">
                  <RingChart
                    used={usageData.quotaReadUsed ?? 0}
                    total={50000}
                    label="Read Quota"
                    color="#8b5cf6"
                  />
                  <RingChart
                    used={usageData.quotaWriteUsed ?? 0}
                    total={20000}
                    label="Write Quota"
                    color="#06b6d4"
                  />
                  <div className="flex flex-col justify-center items-center text-center gap-2">
                    <Database className="h-8 w-8 text-primary/60" />
                    <p className="text-sm font-medium">Session Count</p>
                    <p className="text-2xl font-bold font-mono">{usageData.sessionCount ?? 0}</p>
                  </div>
                  <div className="flex flex-col justify-center items-center text-center gap-2">
                    <HardDrive className="h-8 w-8 text-cyan-500/60" />
                    <p className="text-sm font-medium">Message Count</p>
                    <p className="text-2xl font-bold font-mono">{usageData.messageCount ?? 0}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          ) : (
            <p className="text-sm text-muted-foreground">No usage data available.</p>
          )}
        </section>


        {/* ─── Memory (RAG) ────────────────────────────────────────────────── */}
        <section>
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-widest font-mono mb-4">
            Agent Memory
          </h2>
          {ragLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground text-sm">
              <Loader2 className="h-4 w-4 animate-spin" /> Loading memory stats...
            </div>
          ) : (
            <Card className="border-border/50 bg-[#0d0d0f]">
              <CardContent className="p-6 space-y-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <Brain className="h-5 w-5 text-violet-400" />
                    <div>
                      <p className="text-sm font-medium">
                        {ragStats?.total ?? 0} stored memories
                      </p>
                      <p className="text-xs text-muted-foreground font-mono">
                        {Object.entries(ragStats?.by_type ?? {}).map(([t, n]) => `${n} ${t}`).join(' · ') || 'no memories yet'}
                      </p>
                    </div>
                  </div>
                  {!clearConfirm ? (
                    <button
                      onClick={() => setClearConfirm(true)}
                      className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-destructive transition-colors font-mono px-2 py-1 rounded border border-border/50 hover:border-destructive/50"
                    >
                      <Trash2 className="h-3 w-3" /> clear all
                    </button>
                  ) : (
                    <div className="flex items-center gap-2">
                      <AlertTriangle className="h-3.5 w-3.5 text-amber-400" />
                      <span className="text-xs text-amber-400 font-mono">sure?</span>
                      <button
                        onClick={() => { clearMemory.mutate(undefined); setClearConfirm(false); }}
                        className="text-xs text-destructive font-mono px-2 py-0.5 rounded border border-destructive/40 hover:bg-destructive/10"
                        disabled={clearMemory.isPending}
                      >
                        {clearMemory.isPending ? '...' : 'yes'}
                      </button>
                      <button
                        onClick={() => setClearConfirm(false)}
                        className="text-xs text-muted-foreground font-mono px-2 py-0.5 rounded border border-border/50"
                      >
                        no
                      </button>
                    </div>
                  )}
                </div>
                {ragStats && ragStats.total > 0 && (
                  <div className="grid grid-cols-4 gap-2">
                    {[
                      { label: 'code',       color: 'text-cyan-400',   bg: 'bg-cyan-400/10'   },
                      { label: 'fact',       color: 'text-emerald-400', bg: 'bg-emerald-400/10'},
                      { label: 'preference', color: 'text-violet-400', bg: 'bg-violet-400/10' },
                      { label: 'context',    color: 'text-amber-400',  bg: 'bg-amber-400/10'  },
                    ].map(({ label, color, bg }) => (
                      <div key={label} className={`${bg} rounded-lg p-3 text-center`}>
                        <p className={`text-lg font-bold font-mono ${color}`}>
                          {ragStats.by_type[label] ?? 0}
                        </p>
                        <p className="text-[10px] text-muted-foreground mt-0.5">{label}</p>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          )}
        </section>

      </div>
    </AppLayout>
  );
}
