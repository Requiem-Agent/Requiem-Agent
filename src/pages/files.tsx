import { useState, useRef } from "react";
import { AppLayout } from "@/components/layout";
import { useFiles, useFileContent, useDeleteFile, useUploadFile } from "@/hooks/use-files";
import { useToast } from "@/hooks/use-toast";
import {
  Folder, FileText, FileCode, FileImage, Trash2, Eye,
  Download, RefreshCw, X, Copy, Check, Upload, HardDrive,
  ChevronRight, FolderOpen,
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

export default function FilesPage() {
  const { data: files = [], isLoading, refetch, isFetching } = useFiles();
  const deleteMutation = useDeleteFile();
  const uploadMutation = useUploadFile();
  const { toast } = useToast();
  const fileInputRef = useRef<HTMLInputElement>(null);

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
        {selectedFile ? (
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
        )}
      </div>
    </AppLayout>
  );
}
