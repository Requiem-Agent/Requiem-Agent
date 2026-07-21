import { useState } from "react";
import { AppLayout } from "@/components/layout";
import {
  useWorkspaces, useWorkspaceMutations,
  type WorkspaceMeta,
} from "@/hooks/use-workspaces";
import { useToast } from "@/hooks/use-toast";
import {
  FolderOpen, Plus, Trash2, RefreshCw, GitBranch,
  Loader2, FolderClosed, X, FileCode2,
} from "lucide-react";
import { cn } from "@/lib/utils";

function formatSize(b: number) {
  if (b < 1024) return `${b}B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)}KB`;
  return `${(b / (1024 * 1024)).toFixed(1)}MB`;
}

// ── Create workspace dialog ───────────────────────────────────────────────────
function CreateDialog({ onClose }: { onClose: () => void }) {
  const [name, setName]   = useState("");
  const [desc, setDesc]   = useState("");
  const [url,  setUrl]    = useState("");
  const [mode, setMode]   = useState<"new" | "clone">("new");
  const { create, clone } = useWorkspaceMutations();
  const { toast } = useToast();

  async function handleSubmit() {
    if (!name.trim()) return;
    try {
      const ws = await create.mutateAsync({ name: name.trim(), description: desc.trim() });
      if (mode === "clone" && url.trim()) {
        await clone.mutateAsync({ id: (ws as any).id ?? ws, url: url.trim() });
        toast({ title: "Cloning repo…", description: "Files will appear once git clone finishes." });
      } else {
        toast({ title: "Workspace created", description: name });
      }
      onClose();
    } catch (e: any) {
      toast({ title: "Error", description: e.message, variant: "destructive" });
    }
  }

  const busy = create.isPending || clone.isPending;

  return (
    <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center p-4 bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-sm bg-card border border-border rounded-2xl shadow-2xl overflow-hidden animate-slide-up">
        <div className="flex items-center justify-between px-4 py-3 border-b border-border/50">
          <span className="text-sm font-semibold">New Workspace</span>
          <button onClick={onClose} className="p-1 rounded-lg text-muted-foreground hover:text-foreground transition-colors">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="p-4 space-y-3">
          {/* Mode toggle */}
          <div className="flex rounded-xl border border-border/50 overflow-hidden">
            {(["new", "clone"] as const).map(m => (
              <button key={m} onClick={() => setMode(m)}
                className={cn("flex-1 py-1.5 text-xs font-medium transition-all",
                  mode === m ? "bg-primary/15 text-primary" : "text-muted-foreground hover:bg-white/[0.04]")}>
                {m === "new" ? "Empty" : "Clone Git Repo"}
              </button>
            ))}
          </div>

          <input
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="Workspace name"
            className="w-full bg-input/40 border border-border/50 rounded-xl px-3 py-2.5 text-sm outline-none focus:border-primary/40 placeholder:text-muted-foreground/40"
          />
          <input
            value={desc}
            onChange={e => setDesc(e.target.value)}
            placeholder="Description (optional)"
            className="w-full bg-input/40 border border-border/50 rounded-xl px-3 py-2.5 text-sm outline-none focus:border-primary/40 placeholder:text-muted-foreground/40"
          />
          {mode === "clone" && (
            <input
              value={url}
              onChange={e => setUrl(e.target.value)}
              placeholder="https://github.com/user/repo"
              className="w-full bg-input/40 border border-border/50 rounded-xl px-3 py-2.5 text-sm outline-none focus:border-primary/40 placeholder:text-muted-foreground/40"
            />
          )}
          <button
            onClick={handleSubmit}
            disabled={!name.trim() || busy}
            className="w-full py-2.5 rounded-xl bg-primary text-primary-foreground text-sm font-medium disabled:opacity-40 hover:bg-primary/90 transition-all active:scale-[0.98] shadow-lg shadow-primary/20"
          >
            {busy ? <Loader2 className="h-4 w-4 animate-spin mx-auto" /> : mode === "clone" ? "Create & Clone" : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Workspace card ────────────────────────────────────────────────────────────
function WorkspaceCard({ ws }: { ws: WorkspaceMeta }) {
  const { remove } = useWorkspaceMutations();
  const { toast }  = useToast();

  async function handleDelete() {
    if (!confirm(`Delete workspace "${ws.name}" and all its files?`)) return;
    try {
      await remove.mutateAsync(ws.id);
      toast({ title: "Deleted", description: ws.name });
    } catch (e: any) {
      toast({ title: "Delete failed", description: e.message, variant: "destructive" });
    }
  }

  return (
    <div className="flex items-start gap-3 p-4 rounded-2xl border border-border/40 bg-card/30 group hover:bg-card/60 hover:border-border/70 transition-all animate-slide-up">
      <div className="h-10 w-10 rounded-xl bg-amber-400/10 border border-amber-400/20 flex items-center justify-center shrink-0">
        <FolderOpen className="h-5 w-5 text-amber-400" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-semibold truncate">{ws.name}</p>
        {ws.description && (
          <p className="text-xs text-muted-foreground/60 mt-0.5 truncate">{ws.description}</p>
        )}
        <div className="flex items-center gap-3 mt-1.5">
          <span className="text-[10px] text-muted-foreground/50 font-mono flex items-center gap-1">
            <FileCode2 className="h-3 w-3" /> {ws.file_count} files
          </span>
          <span className="text-[10px] text-muted-foreground/50 font-mono">
            {formatSize(ws.size_bytes)}
          </span>
        </div>
      </div>
      <button
        onClick={handleDelete}
        disabled={remove.isPending}
        className="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-400 hover:bg-rose-400/10 transition-all shrink-0"
      >
        {remove.isPending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Trash2 className="h-3.5 w-3.5" />}
      </button>
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────
export default function WorkspacesPage() {
  const { data: workspaces = [], isLoading, refetch, isFetching } = useWorkspaces();
  const [showCreate, setShowCreate] = useState(false);

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 pt-4 pb-6 space-y-4 max-w-lg mx-auto w-full">

          {/* Header */}
          <div className="flex items-center justify-between animate-slide-up">
            <div className="flex items-center gap-2">
              <FolderClosed className="h-5 w-5 text-primary" />
              <div>
                <h1 className="text-base font-semibold">Projects</h1>
                <p className="text-[10px] text-muted-foreground/50">
                  {workspaces.length} workspace{workspaces.length !== 1 ? "s" : ""}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => refetch()}
                disabled={isFetching}
                className="p-2 rounded-xl text-muted-foreground hover:text-foreground border border-border/50 hover:border-border transition-all"
              >
                <RefreshCw className={cn("h-4 w-4", isFetching && "animate-spin")} />
              </button>
              <button
                onClick={() => setShowCreate(true)}
                className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-md shadow-primary/20"
              >
                <Plus className="h-3.5 w-3.5" /> New
              </button>
            </div>
          </div>

          {/* Workspace list */}
          {isLoading ? (
            <div className="space-y-2">
              {[1,2,3].map(i => <div key={i} className="h-20 rounded-2xl animate-shimmer" />)}
            </div>
          ) : workspaces.length === 0 ? (
            <div className="flex flex-col items-center py-16 gap-4 text-center animate-fade-in">
              <div className="h-14 w-14 rounded-2xl bg-primary/5 border border-primary/15 flex items-center justify-center animate-float">
                <FolderClosed className="h-7 w-7 text-primary/40" />
              </div>
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">No workspaces</p>
                <p className="text-xs text-muted-foreground/50 max-w-52">
                  Create a workspace or clone a GitHub repo to get started.
                </p>
              </div>
              <button
                onClick={() => setShowCreate(true)}
                className="flex items-center gap-2 px-4 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-all active:scale-95"
              >
                <Plus className="h-3.5 w-3.5" /> Create workspace
              </button>
            </div>
          ) : (
            <div className="space-y-2">
              {workspaces.map(ws => (
                <WorkspaceCard key={ws.id} ws={ws} />
              ))}
            </div>
          )}

          {/* Tip */}
          <p className="text-[10px] text-muted-foreground/30 text-center font-mono px-4">
            Select a workspace in the Agent tab to enable filesystem tools
          </p>
        </div>
      </div>

      {showCreate && <CreateDialog onClose={() => setShowCreate(false)} />}
    </AppLayout>
  );
}
