import { useState } from "react";
import { AppLayout } from "@/components/layout";
import { useFiles, useFileContent, useDeleteFile } from "@/hooks/use-files";
import { useToast } from "@/hooks/use-toast";
import {
  Folder, FileText, FileCode, FileImage, Trash2, Eye, Download,
  RefreshCw, HardDrive, ChevronRight, X, Copy, Check,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

function getFileIcon(name: string) {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["rs", "ts", "tsx", "js", "jsx", "py", "go", "cpp", "c", "java"].includes(ext))
    return { Icon: FileCode, color: "text-cyan-400" };
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext))
    return { Icon: FileImage, color: "text-emerald-400" };
  if (["md", "txt", "json", "yaml", "toml"].includes(ext))
    return { Icon: FileText, color: "text-amber-400" };
  return { Icon: FileText, color: "text-muted-foreground" };
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)}MB`;
}

function formatDate(iso: string) {
  try {
    return new Date(iso).toLocaleDateString("en", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  } catch { return iso; }
}

export default function FilesPage() {
  const { data: files = [], isLoading, refetch, isFetching } = useFiles();
  const deleteMutation = useDeleteFile();
  const { toast } = useToast();

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
    await navigator.clipboard.writeText(fileContent.content);
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

  const totalSize = files.reduce((acc, f) => acc + (f.size ?? 0), 0);

  return (
    <AppLayout>
      <div className="flex h-full overflow-hidden">
        {/* Sidebar */}
        <div className="w-64 flex-shrink-0 border-r border-border flex flex-col bg-[#09090b]">
          <div className="flex items-center justify-between px-4 py-3 border-b border-border">
            <div className="flex items-center gap-2">
              <Folder className="h-4 w-4 text-primary" />
              <span className="text-sm font-semibold font-mono">Agent Files</span>
            </div>
            <button
              onClick={() => refetch()}
              disabled={isFetching}
              className="p-1 rounded text-muted-foreground hover:text-foreground transition-colors"
            >
              <RefreshCw className={cn("h-3.5 w-3.5", isFetching && "animate-spin")} />
            </button>
          </div>

          {/* Storage usage */}
          <div className="px-4 py-2 border-b border-border/50">
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <div className="flex items-center gap-1.5">
                <HardDrive className="h-3 w-3" />
                <span>{files.length} files</span>
              </div>
              <span className="font-mono">{formatSize(totalSize)}</span>
            </div>
          </div>

          {/* File list */}
          <div className="flex-1 overflow-y-auto py-1">
            {isLoading ? (
              <div className="flex items-center justify-center py-8 text-muted-foreground text-xs">
                <RefreshCw className="h-3 w-3 animate-spin mr-2" />Loading...
              </div>
            ) : files.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-10 px-4 text-center">
                <Folder className="h-8 w-8 text-muted-foreground/30 mb-2" />
                <p className="text-xs text-muted-foreground">No files yet</p>
                <p className="text-[10px] text-muted-foreground/60 mt-1">Files saved during agent sessions appear here</p>
              </div>
            ) : (
              <div className="space-y-0.5 px-2">
                {files.map((file) => {
                  const { Icon, color } = getFileIcon(file.name);
                  const isSelected = selectedFile === file.name;
                  return (
                    <div
                      key={file.name}
                      onClick={() => setSelectedFile(isSelected ? null : file.name)}
                      className={cn(
                        "group flex items-center gap-2.5 px-2 py-2 rounded-md cursor-pointer transition-all",
                        isSelected
                          ? "bg-primary/10 border border-primary/20"
                          : "hover:bg-white/[0.03] border border-transparent"
                      )}
                    >
                      <Icon className={cn("h-4 w-4 flex-shrink-0", color)} />
                      <div className="flex-1 min-w-0">
                        <p className={cn(
                          "text-xs font-mono truncate",
                          isSelected ? "text-foreground" : "text-muted-foreground group-hover:text-foreground"
                        )}>
                          {file.name}
                        </p>
                        <p className="text-[10px] text-muted-foreground/50 mt-0.5">
                          {formatSize(file.size ?? 0)}
                        </p>
                      </div>
                      <button
                        onClick={(e) => handleDelete(file.name, e)}
                        className="opacity-0 group-hover:opacity-100 p-1 rounded text-muted-foreground hover:text-destructive transition-all"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Content area */}
        <div className="flex-1 flex flex-col overflow-hidden bg-[#07070a]">
          {selectedFile ? (
            <>
              {/* File header */}
              <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-[#09090b]">
                <div className="flex items-center gap-2">
                  <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                  <span className="text-sm font-mono text-foreground">{selectedFile}</span>
                  {fileContent && (
                    <Badge variant="outline" className="font-mono text-[10px] text-muted-foreground">
                      {formatSize(fileContent.content?.length ?? 0)}
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 px-2 text-xs gap-1.5"
                    onClick={handleCopy}
                    disabled={!fileContent}
                  >
                    {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
                    {copied ? "Copied" : "Copy"}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 px-2 text-xs gap-1.5"
                    onClick={handleDownload}
                    disabled={!fileContent}
                  >
                    <Download className="h-3 w-3" />
                    Save
                  </Button>
                  <button
                    onClick={() => setSelectedFile(null)}
                    className="p-1.5 rounded text-muted-foreground hover:text-foreground transition-colors"
                  >
                    <X className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>

              {/* File content */}
              <div className="flex-1 overflow-auto p-4">
                {contentLoading ? (
                  <div className="flex items-center justify-center h-32 text-muted-foreground text-xs">
                    <RefreshCw className="h-3 w-3 animate-spin mr-2" />Loading...
                  </div>
                ) : (
                  <pre className="text-xs font-mono text-foreground/80 whitespace-pre-wrap leading-relaxed">
                    {fileContent?.content ?? ""}
                  </pre>
                )}
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4 text-center px-6">
              <div className="h-16 w-16 rounded-xl bg-primary/5 border border-primary/10 flex items-center justify-center">
                <Eye className="h-7 w-7 text-primary/40" />
              </div>
              <div>
                <p className="text-sm font-medium text-muted-foreground">Select a file to preview</p>
                <p className="text-xs text-muted-foreground/50 mt-1">
                  Click any file in the sidebar to view its content
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </AppLayout>
  );
}
