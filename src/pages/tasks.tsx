import { useState } from "react";
import { AppLayout } from "@/components/layout";
import { useToast } from "@/hooks/use-toast";
import {
  CheckSquare, Circle, AlertCircle, CheckCircle2,
  Loader2, GitBranch, Plus, ChevronRight, Pause,
  Network, Zap, RefreshCw,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL || "";
function getToken() { return sessionStorage.getItem("rq_tok") || localStorage.getItem("requiem_token") || ""; }

async function apiFetch(path: string, opts: RequestInit = {}) {
  const res = await fetch(`${API_BASE}/api${path}`, {
    ...opts,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${getToken()}`,
      ...((opts.headers as Record<string, string>) || {}),
    },
  });
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}

type TaskStatus = "pending" | "in_progress" | "completed" | "failed" | "blocked";

interface Task {
  id: string;
  description: string;
  status: TaskStatus;
  assigned_model?: string;
  priority?: number;
  dependencies?: string[];
  children?: Task[];
}

const STATUS_CFG: Record<TaskStatus, { label: string; Icon: React.ElementType; color: string; bg: string; border: string }> = {
  pending:     { label: "Pending",  Icon: Circle,       color: "text-muted-foreground", bg: "bg-muted/15",         border: "border-border/30" },
  in_progress: { label: "Running",  Icon: Loader2,      color: "text-cyan-400",         bg: "bg-cyan-400/8",       border: "border-cyan-400/20" },
  completed:   { label: "Done",     Icon: CheckCircle2, color: "text-emerald-400",       bg: "bg-emerald-400/8",    border: "border-emerald-400/20" },
  failed:      { label: "Failed",   Icon: AlertCircle,  color: "text-rose-400",          bg: "bg-rose-400/8",       border: "border-rose-400/20" },
  blocked:     { label: "Blocked",  Icon: Pause,        color: "text-amber-400",         bg: "bg-amber-400/8",      border: "border-amber-400/20" },
};

function TaskItem({ task, depth = 0 }: { task: Task; depth?: number }) {
  const [open, setOpen] = useState(true);
  const hasChildren = task.children && task.children.length > 0;
  const cfg = STATUS_CFG[task.status] ?? STATUS_CFG.pending;

  return (
    <div className={cn(depth > 0 && "ml-4 border-l border-border/30 pl-3 mt-1.5")}>
      <div className={cn(
        "flex items-center gap-2.5 rounded-xl px-3 py-2.5 border transition-all group",
        cfg.bg, cfg.border,
      )}>
        {hasChildren ? (
          <button
            onClick={() => setOpen(o => !o)}
            className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
          >
            <ChevronRight className={cn("h-3.5 w-3.5 transition-transform duration-200", open && "rotate-90")} />
          </button>
        ) : (
          <div className="w-3.5 shrink-0" />
        )}

        <cfg.Icon className={cn("h-3.5 w-3.5 shrink-0", cfg.color, task.status === "in_progress" && "animate-spin")} />

        <span className="flex-1 text-xs text-foreground/85 leading-relaxed">{task.description}</span>

        {task.assigned_model && (
          <span className="text-[9px] text-muted-foreground/40 font-mono hidden group-hover:inline shrink-0">
            Requiem Agent 1
          </span>
        )}

        <Badge
          variant="outline"
          className={cn("text-[9px] font-mono shrink-0 px-1.5 py-0", cfg.color, cfg.border)}
        >
          {cfg.label}
        </Badge>
      </div>

      {hasChildren && open && (
        <div className="mt-1">
          {task.children!.map(child => (
            <TaskItem key={child.id} task={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
}

export default function TasksPage() {
  const [description, setDescription] = useState("");
  const [activeTreeId, setActiveTreeId] = useState<string | null>(null);
  const qc = useQueryClient();
  const { toast } = useToast();

  const decomposeMutation = useMutation({
    mutationFn: (desc: string) =>
      apiFetch("/tasks/decompose", {
        method: "POST",
        body: JSON.stringify({ description: desc, owner: "user" }),
      }),
    onSuccess: (data: any) => {
      setActiveTreeId(data.id ?? data.tree_id ?? null);
      qc.invalidateQueries({ queryKey: ["task-tree"] });
    },
    onError: (e: any) => toast({ title: "Failed", description: e.message, variant: "destructive" }),
  });

  const { data: treeData, isLoading: treeLoading, refetch: refetchTree } = useQuery({
    queryKey: ["task-tree", activeTreeId],
    queryFn: () => apiFetch(`/tasks/${activeTreeId}`),
    enabled: !!activeTreeId,
    refetchInterval: (data: any) => {
      if (!data) return false;
      const inProgress = (data?.progress?.in_progress ?? 0) > 0;
      return inProgress ? 3000 : false;
    },
  });

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!description.trim() || decomposeMutation.isPending) return;
    decomposeMutation.mutate(description.trim());
    setDescription("");
  }

  const progress = treeData?.progress ?? { total: 0, completed: 0, in_progress: 0, failed: 0 };
  const pct = progress.total > 0 ? Math.round((progress.completed / progress.total) * 100) : 0;

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-hidden">
        {/* Input area */}
        <div className="shrink-0 px-4 pt-4 pb-3 border-b border-border/40">
          <form onSubmit={handleSubmit} className="flex gap-2">
            <div className="flex-1 relative">
              <input
                value={description}
                onChange={e => setDescription(e.target.value)}
                placeholder="Describe a complex task to decompose…"
                className="w-full bg-card/60 border border-border/60 rounded-xl px-4 py-2.5 text-sm text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:border-primary/40 focus:ring-1 focus:ring-primary/20 transition-all"
              />
            </div>
            <button
              type="submit"
              disabled={!description.trim() || decomposeMutation.isPending}
              className={cn(
                "px-4 py-2.5 rounded-xl text-sm font-medium transition-all active:scale-95",
                description.trim() && !decomposeMutation.isPending
                  ? "bg-primary text-primary-foreground hover:bg-primary/90 shadow-md shadow-primary/20"
                  : "bg-muted text-muted-foreground/40 cursor-not-allowed"
              )}
            >
              {decomposeMutation.isPending
                ? <Loader2 className="h-4 w-4 animate-spin" />
                : <Zap className="h-4 w-4" />
              }
            </button>
          </form>
        </div>

        {/* Task tree */}
        <div className="flex-1 overflow-y-auto min-h-0 px-4 py-3 space-y-3">

          {decomposeMutation.isPending && (
            <div className="flex items-center gap-3 px-4 py-4 rounded-xl border border-primary/20 bg-primary/5 animate-fade-in">
              <Loader2 className="h-4 w-4 animate-spin text-primary shrink-0" />
              <div>
                <p className="text-sm font-medium text-primary">Decomposing task…</p>
                <p className="text-xs text-muted-foreground/60">Breaking down into parallel subtasks</p>
              </div>
            </div>
          )}

          {activeTreeId && treeLoading && (
            <div className="flex justify-center py-8">
              <Loader2 className="h-5 w-5 animate-spin text-primary/60" />
            </div>
          )}

          {treeData && (
            <div className="space-y-3 animate-slide-up">
              {/* Tree header */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <GitBranch className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium text-foreground/80 truncate max-w-[220px]">
                    {treeData.description}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-bold font-mono text-primary">{pct}%</span>
                  <button
                    onClick={() => refetchTree()}
                    className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground border border-border/40 transition-all"
                  >
                    <RefreshCw className="h-3 w-3" />
                  </button>
                </div>
              </div>

              {/* Progress bar */}
              <div className="space-y-1.5">
                <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary rounded-full transition-all duration-700 ease-out"
                    style={{ width: `${pct}%` }}
                  />
                </div>
                <div className="flex items-center gap-3 text-[10px] font-mono">
                  <span className="text-emerald-400">{progress.completed} done</span>
                  <span className="text-cyan-400">{progress.in_progress} running</span>
                  <span className="text-muted-foreground/50">
                    {progress.total - progress.completed - progress.in_progress} pending
                  </span>
                  {progress.failed > 0 && (
                    <span className="text-rose-400">{progress.failed} failed</span>
                  )}
                </div>
              </div>

              {/* Task items */}
              <div className="space-y-1.5">
                {(treeData.tasks ?? []).map((task: Task) => (
                  <TaskItem key={task.id} task={task} />
                ))}
              </div>
            </div>
          )}

          {/* Empty state */}
          {!activeTreeId && !decomposeMutation.isPending && (
            <div className="flex flex-col items-center py-16 gap-4 text-center animate-fade-in">
              <div className="h-14 w-14 rounded-2xl bg-primary/5 border border-primary/15 flex items-center justify-center animate-float">
                <Network className="h-7 w-7 text-primary/40" />
              </div>
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">No active tasks</p>
                <p className="text-xs text-muted-foreground/50 max-w-56 leading-relaxed">
                  Enter a complex request above — the agent will break it into parallel subtasks with dependencies.
                </p>
              </div>
              <div className="grid grid-cols-1 gap-2 w-full max-w-xs">
                {[
                  "Build a full-stack React + Rust app",
                  "Implement authentication system",
                  "Create REST API with tests",
                ].map(tip => (
                  <button
                    key={tip}
                    onClick={() => setDescription(tip)}
                    className="text-left text-xs px-3.5 py-2.5 rounded-xl border border-border/50 bg-card/30 hover:bg-card hover:border-primary/30 transition-all text-muted-foreground hover:text-foreground"
                  >
                    {tip}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </AppLayout>
  );
}
