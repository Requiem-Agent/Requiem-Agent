import { useState } from "react";
import { AppLayout } from "@/components/layout";
import { useToast } from "@/hooks/use-toast";
import {
  CheckSquare, Clock, Play, Pause, RotateCcw, Plus,
  ChevronRight, Circle, AlertCircle, CheckCircle2,
  Loader2, Zap, Network, GitBranch,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const API_BASE = import.meta.env.VITE_API_URL || "";
function getToken() { return localStorage.getItem("requiem_token") || ""; }

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

interface TaskTree {
  id: string;
  description: string;
  owner: string;
  created_at: string;
  tasks: Task[];
  progress: { total: number; completed: number; in_progress: number; failed: number };
}

const STATUS_CONFIG: Record<TaskStatus, { label: string; Icon: React.ElementType; color: string; bg: string }> = {
  pending:     { label: "Pending",     Icon: Circle,       color: "text-muted-foreground", bg: "bg-muted/20" },
  in_progress: { label: "Running",     Icon: Loader2,      color: "text-cyan-400",         bg: "bg-cyan-400/8" },
  completed:   { label: "Done",        Icon: CheckCircle2, color: "text-emerald-400",       bg: "bg-emerald-400/8" },
  failed:      { label: "Failed",      Icon: AlertCircle,  color: "text-destructive",       bg: "bg-destructive/8" },
  blocked:     { label: "Blocked",     Icon: Pause,        color: "text-amber-400",         bg: "bg-amber-400/8" },
};

function TaskItem({ task, depth = 0 }: { task: Task; depth?: number }) {
  const [open, setOpen] = useState(true);
  const hasChildren = task.children && task.children.length > 0;
  const cfg = STATUS_CONFIG[task.status] ?? STATUS_CONFIG.pending;

  return (
    <div className={cn("select-none", depth > 0 && "ml-4 border-l border-border/40 pl-3 mt-1")}>
      <div className={cn(
        "flex items-center gap-2 rounded-lg px-3 py-2.5 transition-colors group",
        cfg.bg, "border border-transparent hover:border-border/30"
      )}>
        {hasChildren && (
          <button onClick={() => setOpen(!open)} className="flex-shrink-0 text-muted-foreground hover:text-foreground">
            <ChevronRight className={cn("h-3.5 w-3.5 transition-transform", open && "rotate-90")} />
          </button>
        )}
        {!hasChildren && <div className="w-3.5 flex-shrink-0" />}
        <cfg.Icon className={cn("h-3.5 w-3.5 flex-shrink-0", cfg.color, task.status === "in_progress" && "animate-spin")} />
        <span className="flex-1 text-xs text-foreground/80 leading-relaxed">{task.description}</span>
        <Badge variant="outline" className={cn("text-[10px] font-mono shrink-0", cfg.color)}>
          {cfg.label}
        </Badge>
      </div>
      {hasChildren && open && (
        <div className="mt-1">
          {task.children!.map(child => <TaskItem key={child.id} task={child} depth={depth + 1} />)}
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
      apiFetch("/tasks/decompose", { method: "POST", body: JSON.stringify({ description: desc, owner: "user" }) }),
    onSuccess: (data) => {
      setActiveTreeId(data.id ?? data.tree_id);
      setDescription("");
      qc.invalidateQueries({ queryKey: ["tasks"] });
    },
    onError: () => toast({ title: "Failed to decompose task", variant: "destructive" }),
  });

  const { data: treeData, isLoading: treeLoading } = useQuery<TaskTree>({
    queryKey: ["tasks", activeTreeId],
    queryFn: () => apiFetch(`/tasks/${activeTreeId}`),
    enabled: !!activeTreeId,
    refetchInterval: (query) => {
      const d = query.state.data as TaskTree | undefined;
      if (!d) return false;
      const hasRunning = d.tasks?.some((t: Task) => t.status === "in_progress");
      return hasRunning ? 2000 : false;
    },
  });

  const progress = treeData?.progress;
  const pct = progress && progress.total > 0
    ? Math.round((progress.completed / progress.total) * 100)
    : 0;

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 md:px-6 py-5 space-y-5 max-w-3xl mx-auto w-full">

          {/* Header */}
          <div>
            <h1 className="text-xl font-bold font-mono tracking-tight flex items-center gap-2">
              <Network className="h-5 w-5 text-primary" />
              Task Engine
            </h1>
            <p className="text-xs text-muted-foreground mt-1">
              Decompose complex requests into parallel task trees
            </p>
          </div>

          {/* Input */}
          <div className="flex gap-2">
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter" && description.trim()) decomposeMutation.mutate(description.trim()); }}
              placeholder="Describe a complex task to decompose..."
              className="font-mono text-sm bg-card/30 border-border/50"
            />
            <Button
              onClick={() => description.trim() && decomposeMutation.mutate(description.trim())}
              disabled={decomposeMutation.isPending || !description.trim()}
              className="gap-2 shrink-0"
            >
              {decomposeMutation.isPending
                ? <Loader2 className="h-3.5 w-3.5 animate-spin" />
                : <Zap className="h-3.5 w-3.5" />}
              Decompose
            </Button>
          </div>

          {/* Task tree */}
          {activeTreeId && (
            <div className="space-y-3">
              {treeLoading ? (
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Loader2 className="h-3 w-3 animate-spin" />
                  Loading task tree...
                </div>
              ) : treeData ? (
                <>
                  {/* Progress bar */}
                  {progress && progress.total > 0 && (
                    <div className="rounded-xl border border-border/50 bg-card/20 p-4 space-y-3">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <GitBranch className="h-4 w-4 text-primary" />
                          <span className="text-sm font-medium truncate">{treeData.description}</span>
                        </div>
                        <span className="text-sm font-bold font-mono text-primary">{pct}%</span>
                      </div>
                      <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                        <div
                          className="h-full bg-primary rounded-full transition-all duration-500"
                          style={{ width: `${pct}%` }}
                        />
                      </div>
                      <div className="flex items-center gap-4 text-xs text-muted-foreground font-mono">
                        <span className="text-emerald-400">{progress.completed} done</span>
                        <span className="text-cyan-400">{progress.in_progress} running</span>
                        <span>{progress.total - progress.completed - progress.in_progress} pending</span>
                        {progress.failed > 0 && <span className="text-destructive">{progress.failed} failed</span>}
                      </div>
                    </div>
                  )}

                  {/* Tasks */}
                  <div className="space-y-1.5">
                    {(treeData.tasks ?? []).map((task: Task) => (
                      <TaskItem key={task.id} task={task} />
                    ))}
                  </div>
                </>
              ) : null}
            </div>
          )}

          {/* Empty state */}
          {!activeTreeId && !decomposeMutation.isPending && (
            <div className="flex flex-col items-center py-14 gap-3 text-center">
              <div className="h-14 w-14 rounded-xl bg-primary/5 border border-primary/10 flex items-center justify-center">
                <CheckSquare className="h-6 w-6 text-primary/40" />
              </div>
              <p className="text-sm text-muted-foreground">No active tasks</p>
              <p className="text-xs text-muted-foreground/50 max-w-64">
                Enter a complex request above and the agent will break it into parallel subtasks
              </p>
            </div>
          )}
        </div>
      </div>
    </AppLayout>
  );
}
