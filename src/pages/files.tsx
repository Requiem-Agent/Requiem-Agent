import { useState, useRef } from "react";
import { AppLayout } from "@/components/layout";
import { useFiles, useFileContent, useDeleteFile, useUploadFile } from "@/hooks/use-files";
import {
  useWorkspaces, useWorkspaceTree, useWorkspaceFile, useWorkspaceMutations,
  type TreeNode,
} from "@/hooks/use-workspaces";
import { useToast } from "@/hooks/use-toast";
import {
  Folder, FileText, FileCode, FileImage, Trash2, Eye,
  Download, RefreshCw, X, Copy, Check, Upload, HardDrive,
  ChevronRight, FolderOpen, FolderClosed, FileCode2,
  ChevronDown, Layers, ArrowLeft, Plus,
} from "lucide-react";
import { cn } from "@/lib/utils";

function getFileIcon(name: string): { Icon: React.ElementType; color: string; ext: string } {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["rs", "ts", "tsx", "js", "jsx", "py", "go", "cpp", "c", "java", "swift"].includes(ext))
    return { Icon: FileCode,  color: "text-cyan-400",    ext };
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext))
    return { Icon: FileImage, color: "text-emerald-400", ext };
  if (["md", "txt", "json", "yaml", "toml", "env"].includes(ext))
    return { Icon: FileText,  color: "text-amber-400",   ext };
  return { Icon: FileText, color: "text-muted-foreground", ext };
}

function formatSize(b: number) {
  if (b < 1024) return `${b}B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)}KB`;
  return `${(b / 1024 / 1024).toFixed(1)}MB`;
}

function formatDate(iso: string) {
  try {
    return new Date(iso).toLocaleDateString("en", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  } catch { return iso; }
}

// ── Recursive workspace tree ──────────────────────────────────────────────────
function WsTreeNode({
  node, wsId, depth = 0, onSelect,
}: {
  node: TreeNode; wsId: string; depth?: number;
  onSelect: (path: string) => void;
}) {
  const [expanded, setExpanded] = useState(depth < 2);
  if (node.type === "dir") return (
    <div>
      <button
        onClick={() => setExpanded(e => !e)}
        className="w-full flex items-center gap-1.5 py-0.5 px-1 rounded hover:bg-white/[0.04] text-xs text-muted-foreground transition-all"
        style={{ paddingLeft: `${4 + depth * 14}px` }}
      >
        {expanded
          ? <FolderOpen className="h-3 w-3 text-amber-400 shrink-0" />
          : <FolderClosed className="h-3 w-3 text-amber-400/70 shrink-0" />}
        <span className="truncate font-medium">{node.name}</span>
        <ChevronDown className={cn("h-2.5 w-2.5 ml-auto shrink-0 text-muted-foreground/40 transition-transform", !expanded && "-rotate-90")} />
      </button>
      {expanded && node.children?.map(c => (
        <WsTreeNode key={c.path} node={c} wsId={wsId} depth={depth + 1} onSelect={onSelect} />
      ))}
    </div>
  );
  return (
    <button
      onClick={() => onSelect(node.path)}
      className="w-full flex items-center gap-1.5 py-0.5 px-1 rounded hover:bg-white/[0.04] text-xs text-muted-foreground/80 hover:text-foreground transition-all text-left"
      style={{ paddingLeft: `${4 + depth * 14}px` }}
    >
      {(() => { const { Icon, color } = getFileIcon(node.name); return <Icon className={cn("h-3 w-3 shrink-0", color)} />; })()}
      <span className="truncate">{node.name}</span>
      {node.size !== undefined && (
        <span className="ml-auto text-[9px] text-muted-foreground/30 shrink-0">{formatSize(node.size)}</span>
      )}
    </button>
  );
}

export default function FilesPage() {
  const { data: files = [], isLoading, refetch, isFetching } = useFiles();
  const deleteMutation = useDeleteFile();
  const uploadMutation = useUploadFile();
  const { toast } = useToast();
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Workspace browsing
  const { data: workspaces = [] } = useWorkspaces();
  const [activeWsId, setActiveWsId] = useState<string | null>(null);
  const { data: wsTree, isLoading: treeLoading, refetch: refetchTree } = useWorkspaceTree(activeWsId ?? "");
  const [wsSelectedPath, setWsSelectedPath] = useState<string | null>(null);
  const { data: wsFileData, isLoading: wsFileLoading } = useWorkspaceFile(
    activeWsId ?? "", wsSelectedPath ?? "", !!(activeWsId && wsSelectedPath)
  );
  const { deleteFile: deleteWsFile } = useWorkspaceMutations();
  const [showWsPicker, setShowWsPicker] = useState(false);

  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const { data: fileContent, isLoading: contentLoading } = useFileContent(
    selectedFile ?? "", !!selectedFile
  );

  async function handleDelete(name: string, e: React.MouseEvent) {
    e.stopPropagation();
    if (!confirm(`Delete "${name}"?`)) return;
    try {
      await deleteMutation.mutateAsync(name);
      if (selectedFile === name) setSelectedFile(null);
      toast({ title: "File deleted" });
    } catch {
      toast({ title: "Delete failed", variant: "destructive" });
    }
  }

  async function handleCopy() {
    if (!fileContent?.content) return;
    await navigator.clipboard.writeText(fileContent.content).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  function handleDownload() {
    if (!fileContent?.content || !selectedFile) return;
    const blob = new Blob([fileContent.content], { type: "text/plain" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = selectedFile;
    a.click();
    URL.revokeObjectURL(a.href);
  }

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const text = await file.text();
    try {
      await uploadMutation.mutateAsync({ name: file.name, content: text });
      toast({ title: "File uploaded", description: file.name });
      refetch();
    } catch {
      toast({ title: "Upload failed", variant: "destructive" });
    }
    e.target.value = "";
  }

  const totalSize = files.reduce((acc, f) => acc + (f.size ?? 0), 0);

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-hidden">
        {/* ── Tab bar: Session Files | Workspaces ── */}
        <div className="shrink-0 flex items-center gap-1 px-3 pt-2 pb-0 border-b border-border/40">
          <button
            onClick={() => { setActiveWsId(null); setWsSelectedPath(null); }}
            className={cn(
              "px-3 py-1.5 text-xs font-medium rounded-t-lg border-b-2 transition-all",
              !activeWsId
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            )}
          >Session Files</button>
          {workspaces.map(w => (
            <button
              key={w.id}
              onClick={() => { setActiveWsId(w.id); setWsSelectedPath(null); setSelectedFile(null); }}
              className={cn(
                "px-3 py-1.5 text-xs font-medium rounded-t-lg border-b-2 transition-all truncate max-w-[100px]",
                activeWsId === w.id
                  ? "border-emerald-400 text-emerald-400"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              )}
            >
              <FolderOpen className="h-3 w-3 inline mr-1 -mt-0.5" />
              {w.name}
            </button>
          ))}
        </div>

        {/* ── Workspace view ── */}
        {activeWsId ? (
          <div className="flex flex-1 min-h-0 overflow-hidden">
            {/* File tree panel */}
            <div className="w-44 shrink-0 border-r border-border/40 overflow-y-auto py-2 px-1">
              <div className="flex items-center justify-between px-2 mb-1">
                <span className="text-[10px] text-muted-foreground/50 uppercase tracking-wider font-mono">Files</span>
                <button
                  onClick={() => refetchTree()}
                  className="p-0.5 rounded text-muted-foreground/40 hover:text-muted-foreground transition-colors"
                >
                  <RefreshCw className={cn("h-3 w-3", treeLoading && "animate-spin")} />
                </button>
              </div>
              {treeLoading ? (
                <div className="flex justify-center py-4"><RefreshCw className="h-4 w-4 animate-spin text-muted-foreground/40" /></div>
              ) : wsTree?.tree?.length ? (
                wsTree.tree.map(node => (
                  <WsTreeNode key={node.path} node={node} wsId={activeWsId} onSelect={setWsSelectedPath} />
                ))
              ) : (
                <p className="text-[10px] text-muted-foreground/40 text-center py-4">Empty workspace</p>
              )}
            </div>
            {/* File content panel */}
            <div className="flex-1 overflow-hidden flex flex-col min-w-0">
              {wsSelectedPath ? (
                <>
                  <div className="shrink-0 flex items-center gap-2 px-3 py-2 border-b border-border/40 bg-card/30">
                    <button onClick={() => setWsSelectedPath(null)} className="p-1 rounded text-muted-foreground/50 hover:text-foreground transition-colors">
                      <ArrowLeft className="h-3.5 w-3.5" />
                    </button>
                    {(() => { const { Icon, color } = getFileIcon(wsSelectedPath.split("/").pop() ?? ""); return <Icon className={cn("h-3.5 w-3.5", color)} />; })()}
                    <span className="text-xs font-mono truncate">{wsSelectedPath}</span>
                    <div className="ml-auto flex gap-1.5">
                      <button
                        onClick={async () => {
                          if (!wsFileData?.content) return;
                          await navigator.clipboard.writeText(wsFileData.content).catch(() => {});
                          toast({ title: "Copied" });
                        }}
                        className="px-2 py-1 rounded text-xs border border-border/40 text-muted-foreground hover:text-foreground transition-all"
                      ><Check className="h-3 w-3 inline" /> Copy</button>
                      <button
                        onClick={() => {
                          if (!confirm(`Delete ${wsSelectedPath}?`)) return;
                          deleteWsFile.mutate({ wsId: activeWsId, path: wsSelectedPath });
                          setWsSelectedPath(null);
                        }}
                        className="px-2 py-1 rounded text-xs border border-rose-500/30 text-rose-400 hover:bg-rose-500/10 transition-all"
                      ><Trash2 className="h-3 w-3 inline" /> Del</button>
                    </div>
                  </div>
                  <div className="flex-1 overflow-auto p-3">
                    {wsFileLoading ? (
                      <div className="flex justify-center py-8"><RefreshCw className="h-4 w-4 animate-spin text-muted-foreground/40" /></div>
                    ) : (
                      <pre className="text-[0.72rem] text-foreground/80 font-mono whitespace-pre-wrap break-words leading-relaxed">
                        {wsFileData?.content ?? ""}
                      </pre>
                    )}
                  </div>
                </>
              ) : (
                <div className="flex items-center justify-center h-full text-xs text-muted-foreground/40">
                  Select a file to view
                </div>
              )}
            </div>
          </div>
        ) : selectedFile ? (
          /* ── File viewer ── */
          <div className="flex flex-col h-full">
            {/* Viewer header */}
            <div className="shrink-0 flex items-center gap-2 px-4 py-3 border-b border-border/50 bg-card/30">
              <button
                onClick={() => setSelectedFile(null)}
                className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-white/[0.05] transition-all"
              >
                <X className="h-4 w-4" />
              </button>
              <div className="flex items-center gap-2 flex-1 min-w-0">
                {(() => { const { Icon, color } = getFileIcon(selectedFile); return <Icon className={cn("h-4 w-4 shrink-0", color)} />; })()}
                <span className="text-sm font-mono font-medium truncate">{selectedFile}</span>
              </div>
              <div className="flex items-center gap-1.5 shrink-0">
                <button
                  onClick={handleCopy}
                  className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs text-muted-foreground hover:text-foreground border border-border/50 hover:border-border transition-all"
                >
                  {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
                  {copied ? "Copied" : "Copy"}
                </button>
                <button
                  onClick={handleDownload}
                  className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs text-muted-foreground hover:text-foreground border border-border/50 hover:border-border transition-all"
                >
                  <Download className="h-3 w-3" />
                  Save
                </button>
              </div>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-auto min-h-0 p-4">
              {contentLoading ? (
                <div className="flex items-center justify-center h-32 gap-2 text-muted-foreground">
                  <RefreshCw className="h-4 w-4 animate-spin" />
                  <span className="text-xs">Loading…</span>
                </div>
              ) : (
                <pre className="code-block text-[0.78rem] text-foreground/80 whitespace-pre-wrap break-words leading-relaxed">
                  {fileContent?.content ?? ""}
                </pre>
              )}
            </div>
          </div>
        ) : (
          /* ── File list ── */
          <div className="flex flex-col h-full overflow-y-auto">
            <div className="px-4 pt-4 pb-6 space-y-4 max-w-lg mx-auto w-full">

              {/* Header */}
              <div className="flex items-center justify-between animate-slide-up">
                <div className="flex items-center gap-2">
                  <FolderOpen className="h-5 w-5 text-primary" />
                  <div>
                    <h1 className="text-base font-semibold">Files</h1>
                    <p className="text-[10px] text-muted-foreground/50">
                      {files.length} files · {formatSize(totalSize)}
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
                    onClick={() => fileInputRef.current?.click()}
                    disabled={uploadMutation.isPending}
                    className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-md shadow-primary/20"
                  >
                    {uploadMutation.isPending
                      ? <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                      : <Upload className="h-3.5 w-3.5" />
                    }
                    Upload
                  </button>
                  <input ref={fileInputRef} type="file" className="hidden" onChange={handleUpload} />
                </div>
              </div>

              {/* File grid */}
              {isLoading ? (
                <div className="space-y-2">
                  {[1,2,3].map(i => (
                    <div key={i} className="h-14 rounded-xl animate-shimmer" />
                  ))}
                </div>
              ) : files.length === 0 ? (
                <div className="flex flex-col items-center py-16 gap-4 text-center animate-fade-in">
                  <div className="h-14 w-14 rounded-2xl bg-primary/5 border border-primary/15 flex items-center justify-center animate-float">
                    <Folder className="h-7 w-7 text-primary/40" />
                  </div>
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-muted-foreground">No files yet</p>
                    <p className="text-xs text-muted-foreground/50 max-w-52">
                      Files generated by the agent or uploaded by you appear here.
                    </p>
                  </div>
                </div>
              ) : (
                <div className="space-y-1.5 stagger">
                  {files.map(file => {
                    const { Icon, color } = getFileIcon(file.name);
                    return (
                      <button
                        key={file.name}
                        onClick={() => setSelectedFile(file.name)}
                        className="w-full flex items-center gap-3 px-3.5 py-3 rounded-xl border border-border/40 bg-card/30 hover:bg-card/60 hover:border-border/70 transition-all group text-left animate-slide-up"
                      >
                        <div className={cn("h-9 w-9 rounded-lg flex items-center justify-center bg-card/80 border border-border/40 shrink-0")}>
                          <Icon className={cn("h-4 w-4", color)} />
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-mono font-medium truncate">{file.name}</p>
                          <p className="text-[10px] text-muted-foreground/50 font-mono mt-0.5">
                            {formatSize(file.size ?? 0)} · {formatDate(file.created_at)}
                          </p>
                        </div>
                        <div className="flex items-center gap-2 shrink-0">
                          <ChevronRight className="h-4 w-4 text-muted-foreground/30 group-hover:text-muted-foreground/60 transition-colors" />
                          <button
                            onClick={e => handleDelete(file.name, e)}
                            className="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-400 hover:bg-rose-400/10 transition-all"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          </div>
        ) : null}
      </div>
    </AppLayout>
  );
}
