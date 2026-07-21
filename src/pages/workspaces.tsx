import { useState } from "react";
import { AppLayout } from "@/components/layout";
import {
  useWorkspaces, useWorkspaceMutations, useWorkspaceTree, useWorkspaceFile,
  type WorkspaceMeta, type TreeNode,
} from "@/hooks/use-workspaces";
import { useToast } from "@/hooks/use-toast";
import {
  FolderOpen, Plus, Trash2, RefreshCw, GitBranch,
  Loader2, FolderClosed, X, FileCode2, ChevronDown,
  ChevronRight, FileText, FileImage, FileCode, Eye,
  Rocket, ExternalLink, Copy, Check, Terminal, ArrowLeft,
  Globe, Code2, Layers,
} from "lucide-react";
import { cn } from "@/lib/utils";

function formatSize(b: number) {
  if (b < 1024) return `${b}B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)}KB`;
  return `${(b / (1024 * 1024)).toFixed(1)}MB`;
}

function getFileIcon(name: string) {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["rs", "ts", "tsx", "js", "jsx", "py", "go"].includes(ext)) return { Icon: FileCode, color: "text-cyan-400" };
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext)) return { Icon: FileImage, color: "text-emerald-400" };
  if (["md", "txt"].includes(ext)) return { Icon: FileText, color: "text-amber-400" };
  return { Icon: FileText, color: "text-muted-foreground/60" };
}

// ── Tree node (recursive) ─────────────────────────────────────────────────────
function TreeItem({
  node, depth = 0, workspaceId, onFileSelect, selectedPath,
}: {
  node: TreeNode; depth?: number; workspaceId: string;
  onFileSelect: (path: string) => void; selectedPath: string | null;
}) {
  const [expanded, setExpanded] = useState(depth < 1);
  if (node.type === "dir") {
    return (
      <div>
        <button
          onClick={() => setExpanded(e => !e)}
          className="w-full flex items-center gap-1.5 py-1 px-2 rounded-lg hover:bg-white/[0.04] text-xs text-muted-foreground transition-all text-left"
          style={{ paddingLeft: `${8 + depth * 16}px` }}
        >
          {expanded
            ? <FolderOpen className="h-3 w-3 text-amber-400 shrink-0" />
            : <FolderClosed className="h-3 w-3 text-amber-400/70 shrink-0" />}
          <span className="truncate font-medium">{node.name}</span>
          <ChevronDown className={cn("h-2.5 w-2.5 ml-auto text-muted-foreground/30 transition-transform shrink-0", !expanded && "-rotate-90")} />
        </button>
        {expanded && node.children?.map((child, i) => (
          <TreeItem key={i} node={child} depth={depth + 1} workspaceId={workspaceId} onFileSelect={onFileSelect} selectedPath={selectedPath} />
        ))}
      </div>
    );
  }
  const { Icon, color } = getFileIcon(node.name);
  const isSelected = selectedPath === node.path;
  return (
    <button
      onClick={() => onFileSelect(node.path ?? node.name)}
      className={cn(
        "w-full flex items-center gap-1.5 py-1 px-2 rounded-lg text-xs transition-all text-left",
        isSelected ? "bg-primary/15 text-primary border-l-2 border-primary" : "text-foreground/70 hover:bg-white/[0.04]"
      )}
      style={{ paddingLeft: `${8 + depth * 16}px` }}
    >
      <Icon className={cn("h-3 w-3 shrink-0", isSelected ? "text-primary" : color)} />
      <span className="truncate">{node.name}</span>
      {node.size !== undefined && (
        <span className="ml-auto text-[9px] text-muted-foreground/30 shrink-0">{formatSize(node.size)}</span>
      )}
    </button>
  );
}

// ── File viewer ────────────────────────────────────────────────────────────────
function FileViewer({ workspaceId, path, onClose }: { workspaceId: string; path: string; onClose: () => void }) {
  const { data, isLoading } = useWorkspaceFile(workspaceId, path, !!path);
  const [copied, setCopied] = useState(false);
  const content = (data as any)?.content ?? "";

  async function handleCopy() {
    try { await navigator.clipboard.writeText(content); setCopied(true); setTimeout(() => setCopied(false), 2000); } catch {}
  }

  const lang = path.split(".").pop()?.toLowerCase() || "";
  const filename = path.split("/").pop() || path;

  return (
    <div className="flex flex-col h-full border-l border-border/40">
      <div className="flex items-center justify-between px-3 py-2 bg-card/30 border-b border-border/40 shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          <FileCode2 className="h-3.5 w-3.5 text-primary/60 shrink-0" />
          <span className="text-xs font-mono truncate text-foreground/80">{filename}</span>
          {lang && <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-primary/10 text-primary/60 border border-primary/20 shrink-0">{lang}</span>}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <button onClick={handleCopy} className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-white/[0.05] transition-all">
            {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
          </button>
          <button onClick={onClose} className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-white/[0.05] transition-all">
            <X className="h-3 w-3" />
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <div className="flex justify-center py-8"><Loader2 className="h-4 w-4 animate-spin text-primary/60" /></div>
        ) : (
          <pre className="text-[0.7rem] font-mono p-3 text-foreground/70 leading-relaxed whitespace-pre-wrap break-words">
            {content || <span className="text-muted-foreground/30 italic">Empty file</span>}
          </pre>
        )}
      </div>
    </div>
  );
}

// ── Workspace expanded view ───────────────────────────────────────────────────
function WorkspaceExpanded({ ws, onClose }: { ws: WorkspaceMeta; onClose: () => void }) {
  const { data: tree, isLoading: treeLoading } = useWorkspaceTree(ws.id);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const nodes: TreeNode[] = Array.isArray(tree) ? tree : [];

  const API_BASE = import.meta.env.VITE_API_URL ?? "https://rayig-dev.hf.space/api";
  const sandboxUrl = `${API_BASE}/workspaces/${ws.id}/preview`;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-background animate-slide-up">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border/50 bg-card/50 shrink-0">
        <div className="flex items-center gap-3">
          <button onClick={onClose} className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-white/[0.05] transition-all">
            <ArrowLeft className="h-4 w-4" />
          </button>
          <div className="h-8 w-8 rounded-xl bg-amber-400/10 border border-amber-400/20 flex items-center justify-center">
            <FolderOpen className="h-4 w-4 text-amber-400" />
          </div>
          <div>
            <p className="text-sm font-semibold">{ws.name}</p>
            <p className="text-[10px] text-muted-foreground/50">{ws.file_count} files · {formatSize(ws.size_bytes)}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <a href={`https://rayig-dev.hf.space`} target="_blank" rel="noopener noreferrer"
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg border border-border/50 text-xs text-muted-foreground hover:text-foreground hover:border-border transition-all">
            <Globe className="h-3 w-3" />Preview
          </a>
        </div>
      </div>

      {/* Content: tree + viewer */}
      <div className="flex-1 overflow-hidden flex">
        {/* File tree */}
        <div className={cn("overflow-y-auto bg-card/20 border-r border-border/40 transition-all", selectedFile ? "w-48 shrink-0" : "flex-1")}>
          <div className="px-3 py-2 flex items-center justify-between sticky top-0 bg-card/50 backdrop-blur-sm border-b border-border/30">
            <span className="text-[10px] font-mono text-muted-foreground/50 uppercase tracking-wider">Files</span>
            <Layers className="h-3 w-3 text-muted-foreground/30" />
          </div>
          {treeLoading ? (
            <div className="p-4 space-y-1">{[1,2,3,4].map(i => <div key={i} className="h-5 rounded animate-shimmer" />)}</div>
          ) : nodes.length === 0 ? (
            <div className="p-6 text-center">
              <Code2 className="h-8 w-8 text-muted-foreground/20 mx-auto mb-2" />
              <p className="text-xs text-muted-foreground/50">No files yet</p>
              <p className="text-[10px] text-muted-foreground/30 mt-1">Chat with the agent to build your project</p>
            </div>
          ) : (
            <div className="p-2 space-y-0.5">
              {nodes.map((node, i) => (
                <TreeItem key={i} node={node} workspaceId={ws.id} onFileSelect={setSelectedFile} selectedPath={selectedFile} />
              ))}
            </div>
          )}
        </div>

        {/* File viewer */}
        {selectedFile && (
          <div className="flex-1 overflow-hidden flex flex-col">
            <FileViewer workspaceId={ws.id} path={selectedFile} onClose={() => setSelectedFile(null)} />
          </div>
        )}
      </div>
    </div>
  );
}

// ── Create workspace dialog ───────────────────────────────────────────────────
function CreateDialog({ onClose }: { onClose: () => void }) {
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [url, setUrl]   = useState("");
  const [mode, setMode] = useState<"new" | "clone">("new");
  const { create, clone } = useWorkspaceMutations();
  const { toast } = useToast();

  async function handleSubmit() {
    if (!name.trim()) return;
    try {
      const ws = await create.mutateAsync({ name: name.trim(), description: desc.trim() });
      if (mode === "clone" && url.trim()) {
        await clone.mutateAsync({ id: (ws as any).id ?? ws, url: url.trim() });
        toast({ title: "Cloning…", description: "Files will appear once clone finishes." });
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
    <div className="fixed inset-0 z-50 flex items-end justify-center p-4 bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-sm bg-card border border-border rounded-2xl shadow-2xl overflow-hidden animate-slide-up">
        <div className="flex items-center justify-between px-4 py-3 border-b border-border/50">
          <span className="text-sm font-semibold">New Workspace</span>
          <button onClick={onClose} className="p-1 rounded-lg text-muted-foreground hover:text-foreground transition-colors">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="p-4 space-y-3">
          <div className="flex rounded-xl border border-border/50 overflow-hidden">
            {(["new", "clone"] as const).map(m => (
              <button key={m} onClick={() => setMode(m)}
                className={cn("flex-1 py-2 text-xs font-medium transition-all",
                  mode === m ? "bg-primary/15 text-primary" : "text-muted-foreground hover:bg-white/[0.04]")}>
                {m === "new" ? "📁 Empty Project" : "🔗 Clone Git Repo"}
              </button>
            ))}
          </div>
          <input value={name} onChange={e => setName(e.target.value)} placeholder="Project name"
            className="w-full bg-input/40 border border-border/50 rounded-xl px-3 py-2.5 text-sm outline-none focus:border-primary/40 placeholder:text-muted-foreground/40" />
          <input value={desc} onChange={e => setDesc(e.target.value)} placeholder="Description (optional)"
            className="w-full bg-input/40 border border-border/50 rounded-xl px-3 py-2.5 text-sm outline-none focus:border-primary/40 placeholder:text-muted-foreground/40" />
          {mode === "clone" && (
            <input value={url} onChange={e => setUrl(e.target.value)} placeholder="https://github.com/user/repo"
              className="w-full bg-input/40 border border-border/50 rounded-xl px-3 py-2.5 text-sm outline-none focus:border-primary/40 placeholder:text-muted-foreground/40" />
          )}
          <button onClick={handleSubmit} disabled={!name.trim() || busy}
            className="w-full py-2.5 rounded-xl bg-primary text-primary-foreground text-sm font-medium disabled:opacity-40 hover:bg-primary/90 transition-all active:scale-[0.98] shadow-lg shadow-primary/20">
            {busy ? <Loader2 className="h-4 w-4 animate-spin mx-auto" /> : mode === "clone" ? "Create & Clone" : "Create Project"}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Workspace card ─────────────────────────────────────────────────────────────
function WorkspaceCard({ ws, onOpen }: { ws: WorkspaceMeta; onOpen: (ws: WorkspaceMeta) => void }) {
  const { remove } = useWorkspaceMutations();
  const { toast } = useToast();

  async function handleDelete(e: React.MouseEvent) {
    e.stopPropagation();
    try {
      await remove.mutateAsync(ws.id);
      toast({ title: "Deleted", description: ws.name });
    } catch (e: any) {
      toast({ title: "Delete failed", description: e.message, variant: "destructive" });
    }
  }

  return (
    <div
      onClick={() => onOpen(ws)}
      className="flex items-start gap-3 p-4 rounded-2xl border border-border/40 bg-card/30 group hover:bg-card/60 hover:border-amber-400/30 transition-all animate-slide-up cursor-pointer active:scale-[0.99]"
    >
      <div className="h-10 w-10 rounded-xl bg-amber-400/10 border border-amber-400/20 flex items-center justify-center shrink-0 group-hover:bg-amber-400/15 transition-all">
        <FolderOpen className="h-5 w-5 text-amber-400" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="text-sm font-semibold truncate">{ws.name}</p>
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground/30 shrink-0 group-hover:text-primary/50 transition-all group-hover:translate-x-0.5" />
        </div>
        {ws.description && <p className="text-xs text-muted-foreground/60 mt-0.5 truncate">{ws.description}</p>}
        <div className="flex items-center gap-3 mt-1.5">
          <span className="text-[10px] text-muted-foreground/50 font-mono flex items-center gap-1">
            <FileCode2 className="h-3 w-3" />{ws.file_count ?? 0} files
          </span>
          <span className="text-[10px] text-muted-foreground/50 font-mono">{formatSize(ws.size_bytes ?? 0)}</span>
          {ws.git_url && (
            <span className="text-[10px] text-primary/40 flex items-center gap-0.5 font-mono">
              <GitBranch className="h-2.5 w-2.5" />git
            </span>
          )}
        </div>
      </div>
      <button
        onClick={handleDelete}
        disabled={remove.isPending}
        className="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-400 hover:bg-rose-400/10 transition-all shrink-0 z-10"
      >
        {remove.isPending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Trash2 className="h-3.5 w-3.5" />}
      </button>
    </div>
  );
}

// ── Main page ──────────────────────────────────────────────────────────────────
export default function WorkspacesPage() {
  const { data: workspaces = [], isLoading, refetch, isFetching } = useWorkspaces();
  const [showCreate, setShowCreate] = useState(false);
  const [expandedWs, setExpandedWs] = useState<WorkspaceMeta | null>(null);

  if (expandedWs) {
    return <WorkspaceExpanded ws={expandedWs} onClose={() => setExpandedWs(null)} />;
  }

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 pt-4 pb-6 space-y-4 max-w-lg mx-auto w-full">

          <div className="flex items-center justify-between animate-slide-up">
            <div className="flex items-center gap-2">
              <FolderClosed className="h-5 w-5 text-primary" />
              <div>
                <h1 className="text-base font-semibold">Projects</h1>
                <p className="text-[10px] text-muted-foreground/50">
                  {workspaces.length} workspace{workspaces.length !== 1 ? "s" : ""} · tap to browse files
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <button onClick={() => refetch()} disabled={isFetching}
                className="p-2 rounded-xl text-muted-foreground hover:text-foreground border border-border/50 hover:border-border transition-all">
                <RefreshCw className={cn("h-4 w-4", isFetching && "animate-spin")} />
              </button>
              <button onClick={() => setShowCreate(true)}
                className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-md shadow-primary/20">
                <Plus className="h-3.5 w-3.5" />New
              </button>
            </div>
          </div>

          {isLoading ? (
            <div className="space-y-2">{[1,2,3].map(i => <div key={i} className="h-20 rounded-2xl animate-shimmer" />)}</div>
          ) : workspaces.length === 0 ? (
            <div className="flex flex-col items-center py-16 gap-4 text-center animate-fade-in">
              <div className="h-14 w-14 rounded-2xl bg-primary/5 border border-primary/15 flex items-center justify-center animate-float">
                <FolderClosed className="h-7 w-7 text-primary/40" />
              </div>
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">No projects yet</p>
                <p className="text-xs text-muted-foreground/50 max-w-56">Create a project and let the agent build it for you.</p>
              </div>
              <button onClick={() => setShowCreate(true)}
                className="flex items-center gap-2 px-4 py-2 rounded-xl border border-primary/30 text-primary text-xs hover:bg-primary/5 transition-all">
                <Plus className="h-3.5 w-3.5" />Create First Project
              </button>
            </div>
          ) : (
            <div className="space-y-3 stagger">
              {workspaces.map(ws => (
                <WorkspaceCard key={ws.id} ws={ws} onOpen={setExpandedWs} />
              ))}
            </div>
          )}
        </div>
      </div>
      {showCreate && <CreateDialog onClose={() => setShowCreate(false)} />}
    </AppLayout>
  );
}
