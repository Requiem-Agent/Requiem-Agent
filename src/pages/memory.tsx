import { useState } from "react";
import { AppLayout } from "@/components/layout";
import { useRagStats, useMemories, useClearMemory, useDeleteMemory, type MemoryType } from "@/hooks/use-memory";
import { useToast } from "@/hooks/use-toast";
import {
  Brain, Code2, BookOpen, Star, Layers, AlertCircle,
  Trash2, RefreshCw, AlertTriangle, Filter, ChevronDown,
  TrendingUp, Database, Zap,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const TYPE_CONFIG: Record<MemoryType, { label: string; Icon: React.ElementType; color: string; bg: string; border: string }> = {
  code:       { label: "Code",       Icon: Code2,       color: "text-cyan-400",    bg: "bg-cyan-400/8",    border: "border-cyan-400/20" },
  fact:       { label: "Fact",       Icon: BookOpen,    color: "text-emerald-400", bg: "bg-emerald-400/8", border: "border-emerald-400/20" },
  preference: { label: "Preference", Icon: Star,        color: "text-violet-400",  bg: "bg-violet-400/8",  border: "border-violet-400/20" },
  context:    { label: "Context",    Icon: Layers,      color: "text-amber-400",   bg: "bg-amber-400/8",   border: "border-amber-400/20" },
  error:      { label: "Error",      Icon: AlertCircle, color: "text-destructive", bg: "bg-destructive/8", border: "border-destructive/20" },
};

const PRIORITY_CONFIG: Record<string, { label: string; color: string; dot: string }> = {
  critical: { label: "critical", color: "text-destructive", dot: "bg-destructive" },
  high:     { label: "high",     color: "text-amber-400",   dot: "bg-amber-400" },
  medium:   { label: "medium",   color: "text-cyan-400",    dot: "bg-cyan-400" },
  low:      { label: "low",      color: "text-muted-foreground", dot: "bg-muted-foreground" },
};

function formatDate(iso: string) {
  try { return new Date(iso).toLocaleDateString("en", { month: "short", day: "numeric" }); }
  catch { return iso; }
}

function StatCard({ label, value, icon: Icon, color }: { label: string; value: number; icon: React.ElementType; color: string }) {
  return (
    <div className={cn("rounded-xl border p-4 bg-card/30 flex items-center gap-3", color.replace("text-", "border-").replace("400", "400/20"))}>
      <div className={cn("h-9 w-9 rounded-lg flex items-center justify-center", color.replace("text-", "bg-").replace("400", "400/10"))}>
        <Icon className={cn("h-4 w-4", color)} />
      </div>
      <div>
        <p className="text-2xl font-bold font-mono tracking-tight">{value}</p>
        <p className="text-xs text-muted-foreground">{label}</p>
      </div>
    </div>
  );
}

export default function MemoryPage() {
  const { data: stats, isLoading: statsLoading, refetch: refetchStats, isFetching: statsFetching } = useRagStats();
  const [activeType, setActiveType] = useState<MemoryType | "all">("all");
  const { data: memories, isLoading: memoriesLoading, refetch: refetchMems } = useMemories(
    activeType !== "all" ? activeType : undefined
  );
  const clearMemory = useClearMemory();
  const deleteMemory = useDeleteMemory();
  const { toast } = useToast();
  const [clearConfirm, setClearConfirm] = useState(false);

  async function handleClear() {
    try {
      await clearMemory.mutateAsync(undefined);
      setClearConfirm(false);
      refetchStats();
      refetchMems();
      toast({ title: "Memory cleared" });
    } catch {
      toast({ title: "Failed to clear memory", variant: "destructive" });
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteMemory.mutateAsync(id);
      refetchStats();
      refetchMems();
    } catch {
      toast({ title: "Failed to delete memory", variant: "destructive" });
    }
  }

  const memoryList: any[] = Array.isArray(memories) ? memories : (memories as any)?.memories ?? [];
  const total = stats?.total ?? 0;

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 md:px-6 py-5 space-y-6 max-w-3xl mx-auto w-full">

          {/* Header */}
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl font-bold font-mono tracking-tight flex items-center gap-2">
                <Brain className="h-5 w-5 text-primary" />
                Agent Memory
              </h1>
              <p className="text-xs text-muted-foreground mt-1">RAG-based persistent memory across sessions</p>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => { refetchStats(); refetchMems(); }}
                disabled={statsFetching}
                className="p-2 rounded-md text-muted-foreground hover:text-foreground hover:bg-white/5 transition-colors"
              >
                <RefreshCw className={cn("h-4 w-4", statsFetching && "animate-spin")} />
              </button>
              {!clearConfirm ? (
                <button
                  onClick={() => setClearConfirm(true)}
                  disabled={total === 0}
                  className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-destructive transition-colors font-mono px-2 py-1 rounded border border-border/50 hover:border-destructive/50 disabled:opacity-30"
                >
                  <Trash2 className="h-3 w-3" /> clear all
                </button>
              ) : (
                <div className="flex items-center gap-2">
                  <AlertTriangle className="h-3.5 w-3.5 text-amber-400" />
                  <span className="text-xs text-amber-400 font-mono">sure?</span>
                  <button
                    onClick={handleClear}
                    className="text-xs text-destructive font-mono px-2 py-0.5 rounded border border-destructive/40 hover:bg-destructive/10"
                    disabled={clearMemory.isPending}
                  >
                    {clearMemory.isPending ? "..." : "yes"}
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
          </div>

          {/* Stats grid */}
          {statsLoading ? (
            <div className="grid grid-cols-2 gap-3">
              {[...Array(4)].map((_, i) => (
                <div key={i} className="h-20 rounded-xl bg-card/20 animate-pulse" />
              ))}
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-3">
              <StatCard label="Total memories" value={total} icon={Database} color="text-primary" />
              <StatCard label="High priority" value={(stats?.by_priority?.critical ?? 0) + (stats?.by_priority?.high ?? 0)} icon={Zap} color="text-amber-400" />
              <StatCard label="Code snippets" value={stats?.by_type?.code ?? 0} icon={Code2} color="text-cyan-400" />
              <StatCard label="Context entries" value={stats?.by_type?.context ?? 0} icon={Layers} color="text-violet-400" />
            </div>
          )}

          {/* Type breakdown */}
          {total > 0 && (
            <div className="space-y-2">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">By Type</p>
              <div className="grid grid-cols-5 gap-2">
                {Object.entries(TYPE_CONFIG).map(([type, cfg]) => {
                  const count = stats?.by_type?.[type] ?? 0;
                  const pct = total > 0 ? (count / total) * 100 : 0;
                  return (
                    <button
                      key={type}
                      onClick={() => setActiveType(activeType === type as MemoryType ? "all" : type as MemoryType)}
                      className={cn(
                        "rounded-lg p-2.5 text-center border transition-all",
                        activeType === type
                          ? `${cfg.bg} ${cfg.border} ${cfg.color}`
                          : "bg-card/20 border-border/30 text-muted-foreground hover:border-border"
                      )}
                    >
                      <cfg.Icon className="h-3.5 w-3.5 mx-auto mb-1" />
                      <p className="text-base font-bold font-mono">{count}</p>
                      <p className="text-[9px] mt-0.5 opacity-70">{cfg.label}</p>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Memory list */}
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                {activeType === "all" ? "All Memories" : TYPE_CONFIG[activeType as MemoryType]?.label}
              </p>
              <Badge variant="outline" className="font-mono text-[10px]">{memoryList.length}</Badge>
            </div>

            {memoriesLoading ? (
              <div className="space-y-2">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="h-16 rounded-lg bg-card/20 animate-pulse" />
                ))}
              </div>
            ) : memoryList.length === 0 ? (
              <div className="flex flex-col items-center py-10 text-center gap-2">
                <Brain className="h-8 w-8 text-muted-foreground/20" />
                <p className="text-sm text-muted-foreground">
                  {total === 0 ? "No memories stored yet" : "No memories in this category"}
                </p>
                <p className="text-xs text-muted-foreground/50">
                  Memories are automatically stored during chat sessions
                </p>
              </div>
            ) : (
              <div className="space-y-2">
                {memoryList.slice(0, 50).map((mem: any) => {
                  const type = mem.memory_type as MemoryType;
                  const cfg = TYPE_CONFIG[type] ?? TYPE_CONFIG.context;
                  const pri = PRIORITY_CONFIG[mem.priority] ?? PRIORITY_CONFIG.low;
                  return (
                    <div
                      key={mem.id}
                      className={cn(
                        "group rounded-lg border p-3 transition-all hover:border-border",
                        "bg-card/20 border-border/30"
                      )}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex items-start gap-2.5 flex-1 min-w-0">
                          <cfg.Icon className={cn("h-3.5 w-3.5 mt-0.5 flex-shrink-0", cfg.color)} />
                          <p className="text-xs text-foreground/80 leading-relaxed line-clamp-2 flex-1">
                            {mem.content}
                          </p>
                        </div>
                        <button
                          onClick={() => handleDelete(mem.id)}
                          className="opacity-0 group-hover:opacity-100 p-1 rounded text-muted-foreground hover:text-destructive transition-all flex-shrink-0"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      </div>
                      <div className="flex items-center gap-2 mt-2">
                        <div className={cn("h-1.5 w-1.5 rounded-full", pri.dot)} />
                        <span className={cn("text-[10px] font-mono", pri.color)}>{pri.label}</span>
                        <span className="text-[10px] text-muted-foreground/40 ml-auto">{formatDate(mem.created_at)}</span>
                        {mem.access_count > 0 && (
                          <span className="text-[10px] text-muted-foreground/40 flex items-center gap-0.5">
                            <TrendingUp className="h-2.5 w-2.5" />
                            {mem.access_count}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>
    </AppLayout>
  );
}
