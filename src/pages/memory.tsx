import { useState } from "react";
import { AppLayout } from "@/components/layout";
import { useRagStats, useMemories, useClearMemory, useDeleteMemory, type MemoryType } from "@/hooks/use-memory";
import { useToast } from "@/hooks/use-toast";
import {
  Brain, Code2, BookOpen, Star, Layers, AlertCircle, Trash2,
  RefreshCw, AlertTriangle, TrendingUp, Database, Filter, X,
} from "lucide-react";
import { cn } from "@/lib/utils";

const TYPE_CONFIG: Record<MemoryType, { label: string; Icon: React.ElementType; color: string; bg: string; border: string }> = {
  code:       { label: "Code",       Icon: Code2,       color: "text-cyan-400",    bg: "bg-cyan-400/8",    border: "border-cyan-400/20" },
  fact:       { label: "Fact",       Icon: BookOpen,    color: "text-emerald-400", bg: "bg-emerald-400/8", border: "border-emerald-400/20" },
  preference: { label: "Preference", Icon: Star,        color: "text-violet-400",  bg: "bg-violet-400/8",  border: "border-violet-400/20" },
  context:    { label: "Context",    Icon: Layers,      color: "text-amber-400",   bg: "bg-amber-400/8",   border: "border-amber-400/20" },
  error:      { label: "Error",      Icon: AlertCircle, color: "text-rose-400",    bg: "bg-rose-400/8",    border: "border-rose-400/20" },
};

const PRIORITY_DOT: Record<string, string> = {
  critical: "bg-rose-400",
  high:     "bg-amber-400",
  medium:   "bg-cyan-400",
  low:      "bg-muted-foreground/40",
};

function formatDate(iso: string) {
  try { return new Date(iso).toLocaleDateString("en", { month: "short", day: "numeric" }); }
  catch { return iso; }
}

export default function MemoryPage() {
  const { data: stats, isLoading: statsLoading, refetch: refetchStats, isFetching } = useRagStats();
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
      toast({ title: "Failed", variant: "destructive" });
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteMemory.mutateAsync(id);
      refetchStats();
      refetchMems();
    } catch {
      toast({ title: "Failed to delete", variant: "destructive" });
    }
  }

  const memList: any[] = Array.isArray(memories) ? memories : (memories as any)?.memories ?? [];
  const total = stats?.total ?? 0;

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 pt-4 pb-6 space-y-5 max-w-lg mx-auto w-full">

          {/* Header */}
          <div className="flex items-center justify-between animate-slide-up">
            <div className="flex items-center gap-2.5">
              <div className="h-9 w-9 rounded-xl bg-violet-400/10 border border-violet-400/20 flex items-center justify-center">
                <Brain className="h-4.5 w-4.5 text-violet-400" />
              </div>
              <div>
                <h1 className="text-base font-semibold">Memory</h1>
                <p className="text-[10px] text-muted-foreground/50">RAG knowledge store</p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => { refetchStats(); refetchMems(); }}
                disabled={isFetching}
                className="p-2 rounded-xl text-muted-foreground hover:text-foreground border border-border/50 transition-all"
              >
                <RefreshCw className={cn("h-4 w-4", isFetching && "animate-spin")} />
              </button>
            </div>
          </div>

          {/* Stats cards */}
          {!statsLoading && stats && (
            <div className="grid grid-cols-2 gap-2.5 animate-slide-up" style={{ animationDelay: "40ms" }}>
              <div className="rounded-xl border border-violet-400/20 bg-violet-400/5 p-3.5 flex items-center gap-3">
                <Database className="h-5 w-5 text-violet-400 shrink-0" />
                <div>
                  <p className="text-xl font-bold font-mono leading-none">{total}</p>
                  <p className="text-[10px] text-muted-foreground/60 mt-0.5">total entries</p>
                </div>
              </div>
              <div className="rounded-xl border border-cyan-400/20 bg-cyan-400/5 p-3.5 flex items-center gap-3">
                <TrendingUp className="h-5 w-5 text-cyan-400 shrink-0" />
                <div>
                  <p className="text-xl font-bold font-mono leading-none">{(stats as any).total_access_count ?? 0}</p>
                  <p className="text-[10px] text-muted-foreground/60 mt-0.5">total accesses</p>
                </div>
              </div>
            </div>
          )}

          {/* Type breakdown */}
          {total > 0 && (
            <div className="grid grid-cols-5 gap-1.5 animate-slide-up" style={{ animationDelay: "80ms" }}>
              {Object.entries(TYPE_CONFIG).map(([type, cfg]) => {
                const count = (stats as any)?.by_type?.[type] ?? 0;
                return (
                  <button
                    key={type}
                    onClick={() => setActiveType(activeType === type ? "all" : type as MemoryType)}
                    className={cn(
                      "rounded-xl p-2.5 text-center border transition-all",
                      activeType === type
                        ? cn(cfg.bg, cfg.border, cfg.color)
                        : "border-border/30 bg-card/20 text-muted-foreground hover:border-border/60"
                    )}
                  >
                    <p className="text-base font-bold font-mono leading-none">{count}</p>
                    <p className="text-[9px] mt-0.5 truncate">{cfg.label}</p>
                  </button>
                );
              })}
            </div>
          )}

          {/* Clear action */}
          {total > 0 && (
            <div className="flex items-center justify-between px-1 animate-fade-in">
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <Filter className="h-3 w-3" />
                {activeType !== "all" ? (
                  <span className="flex items-center gap-1">
                    Filtering: {TYPE_CONFIG[activeType as MemoryType]?.label}
                    <button onClick={() => setActiveType("all")} className="text-muted-foreground/50 hover:text-foreground ml-0.5">
                      <X className="h-3 w-3" />
                    </button>
                  </span>
                ) : (
                  <span>All types</span>
                )}
              </div>

              {!clearConfirm ? (
                <button
                  onClick={() => setClearConfirm(true)}
                  className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-rose-400 font-mono px-2 py-1 rounded-lg border border-border/40 hover:border-rose-400/30 transition-all"
                >
                  <Trash2 className="h-3 w-3" /> clear all
                </button>
              ) : (
                <div className="flex items-center gap-1.5">
                  <AlertTriangle className="h-3 w-3 text-amber-400" />
                  <span className="text-[10px] text-amber-400 font-mono">sure?</span>
                  <button
                    onClick={handleClear}
                    disabled={clearMemory.isPending}
                    className="text-[10px] text-rose-400 font-mono px-2 py-0.5 rounded border border-rose-400/40 hover:bg-rose-400/10"
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
              )}
            </div>
          )}

          {/* Memory list */}
          {memoriesLoading ? (
            <div className="space-y-2">
              {[1,2,3,4].map(i => <div key={i} className="h-16 rounded-xl animate-shimmer" />)}
            </div>
          ) : memList.length === 0 ? (
            <div className="flex flex-col items-center py-14 gap-4 text-center animate-fade-in">
              <div className="h-14 w-14 rounded-2xl bg-violet-400/5 border border-violet-400/15 flex items-center justify-center animate-float">
                <Brain className="h-7 w-7 text-violet-400/40" />
              </div>
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">No memories yet</p>
                <p className="text-xs text-muted-foreground/50 max-w-52 leading-relaxed">
                  The agent automatically stores useful context from your conversations.
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-2 stagger">
              {memList.slice(0, 50).map((mem: any) => {
                const type = mem.memory_type as MemoryType;
                const cfg = TYPE_CONFIG[type] ?? TYPE_CONFIG.context;
                return (
                  <div
                    key={mem.id}
                    className={cn(
                      "group rounded-xl border p-3.5 transition-all hover:border-border",
                      "bg-card/20 border-border/30 animate-slide-up"
                    )}
                  >
                    <div className="flex items-start gap-2.5">
                      <div className={cn("h-6 w-6 rounded-lg flex items-center justify-center shrink-0 mt-0.5", cfg.bg, cfg.border, "border")}>
                        <cfg.Icon className={cn("h-3 w-3", cfg.color)} />
                      </div>
                      <p className="flex-1 text-xs text-foreground/80 leading-relaxed line-clamp-3 min-w-0">
                        {mem.content}
                      </p>
                      <button
                        onClick={() => handleDelete(mem.id)}
                        className="opacity-0 group-hover:opacity-100 p-1 rounded text-muted-foreground hover:text-rose-400 transition-all shrink-0"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                    <div className="flex items-center gap-2 mt-2 pl-8">
                      <span className={cn("h-1.5 w-1.5 rounded-full shrink-0", PRIORITY_DOT[mem.priority] || PRIORITY_DOT.low)} />
                      <span className="text-[10px] text-muted-foreground/40 font-mono">{mem.priority}</span>
                      <span className="text-[10px] text-muted-foreground/30 font-mono ml-auto">{formatDate(mem.created_at)}</span>
                      {mem.access_count > 0 && (
                        <span className="text-[10px] text-muted-foreground/30 flex items-center gap-0.5 font-mono">
                          <TrendingUp className="h-2.5 w-2.5" />{mem.access_count}
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
    </AppLayout>
  );
}
