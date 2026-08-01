import { useState, useRef, useEffect, useCallback } from "react";
import { AppLayout } from "@/components/layout";
import {
  useSessions,
  useSessionMutations,
  useMessageMutations,
  useMessages,
} from "@/hooks/use-sessions";
import { ROLE_MODEL_MAP, FREE_ZEN_MODELS } from "@/hooks/use-system";
import { FormattedMessage } from "@/components/message-formatter";
import { streamZenChat, cleanDisplayText } from "@/lib/zen-client";
import { Textarea } from "@/components/ui/textarea";
import { SessionMode, SessionEffort } from "@workspace/api-client-react";
import { useToast } from "@/hooks/use-toast";
import {
  Terminal, Plus, X, Command, Code2, Settings2, Palette,
  Bug, Search, Map, Shield, Loader2, BrainCircuit,
  Wrench, CheckCircle2, ChevronDown, Zap, RotateCcw,
  Bot, Sparkles, ArrowUp, FolderOpen, FolderClosed,
  ChevronRight, FileCode2, GitBranch,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { fetchRagContext, autoStoreMemory } from "@/hooks/use-memory";
import {
  useWorkspaces,
  useWorkspaceTree,
  streamAgentChat,
  type AgentChatEvent,
  type TreeNode,
} from "@/hooks/use-workspaces";

// ── Tool-Use Card ─────────────────────────────────────────────────────────────
// Tool icon map
const TOOL_COLORS: Record<string, string> = {
  ws_read:   "text-neutral-600",   ws_write:  "text-neutral-700",
  ws_edit:   "text-neutral-500",  ws_delete: "text-red-600",
  ws_tree:   "text-neutral-800", ws_glob:   "text-neutral-500",
  ws_grep:   "text-neutral-600",  ws_mkdir:  "text-neutral-700",
  ws_bash:   "text-neutral-500",
};
const TOOL_BORDER: Record<string, string> = {
  ws_read:   "border-neutral-300 bg-neutral-100",
  ws_write:  "border-neutral-300 bg-neutral-100",
  ws_edit:   "border-neutral-300 bg-neutral-100",
  ws_delete: "border-red-200 bg-red-50",
  ws_tree:   "border-neutral-300 bg-neutral-100",
  ws_glob:   "border-neutral-300 bg-neutral-100",
  ws_grep:   "border-neutral-300 bg-neutral-100",
  ws_mkdir:  "border-neutral-300 bg-neutral-100",
  ws_bash:   "border-neutral-300 bg-neutral-100",
};

function ToolUseCard({ event, isStreaming: streamActive = false }: { event: AgentChatEvent; isStreaming?: boolean }) {
  const [open, setOpen] = useState(false);

  if (event.type === "thinking") {
    // Sanitize: remove any model names that might appear
    const raw = (event.content ?? "")
      .replace(/deepseek[^\s,]*/gi, "")
      .replace(/mimo[^\s,]*/gi, "")
      .replace(/hy3[^\s,]*/gi, "")
      .replace(/nemotron[^\s,]*/gi, "")
      .replace(/north-mini[^\s,]*/gi, "")
      .replace(/big-pickle[^\s,]*/gi, "")
      .replace(/gpt-[^\s,]*/gi, "")
      .replace(/claude[^\s,]*/gi, "")
      .replace(/\s{2,}/g, " ")
      .trim();
    // Truncate long thinking text at 80 chars
    const thinkingText = raw.length > 80 ? raw.slice(0, 80) + "…" : (raw || "processing…");
    return (
      <div className="thinking-box flex items-center gap-2 py-1.5 px-3 rounded-xl bg-neutral-100 border border-neutral-300 text-xs font-mono text-neutral-700 animate-fade-in">
        <BrainCircuit className="h-3 w-3 shrink-0 animate-pulse" />
        <span className="truncate">{thinkingText}</span>
      </div>
    );
  }

  if (event.type === "memory_hit") return (
    <div className="flex items-center gap-2 py-1.5 px-3 rounded-xl bg-neutral-100 border border-neutral-300 text-xs font-mono text-neutral-600 animate-fade-in">
      <BrainCircuit className="h-3 w-3 shrink-0" />
      <span className="font-semibold">Memory</span>
      <span className="text-muted-foreground/60">
        {event.count} ذاكرة ذات صلة مُدرجة
      </span>
    </div>
  );

  if (event.type === "file_written") {
    const fw = event as any;
    const filePath = fw.path ?? fw.filename ?? "file";
    const actionLabel = fw.action === "ws_write" ? "تم الإنشاء" : fw.action === "ws_edit" ? "تم التعديل" : "تم الكتابة";
    return (
      <div className="flex items-center gap-2 py-1.5 px-3 rounded-xl bg-neutral-100 border border-neutral-300 text-xs font-mono text-neutral-700 animate-fade-in">
        <CheckCircle2 className="h-3 w-3 shrink-0" />
        <span className="font-semibold">{actionLabel}</span>
        <span className="text-neutral-500 truncate">{String(filePath)}</span>
        {fw.lines > 0 && (
          <span className="ml-auto text-neutral-400 shrink-0">{fw.lines}L</span>
        )}
      </div>
    );
  }

  if (event.type === "tool_use") {
    const tool = event.tool ?? "";
    const colorCls  = TOOL_COLORS[tool]  ?? "text-neutral-600";
    const borderCls = TOOL_BORDER[tool]  ?? "border-neutral-300 bg-neutral-100";
    const detail = event.input?.path ?? event.input?.command ?? event.input?.pattern ?? "";
    // Truncate detail preview
    const detailPreview = detail ? String(detail).slice(0, 45) + (String(detail).length > 45 ? "…" : "") : "";
    return (
      <div className={cn("rounded-xl border overflow-hidden animate-fade-in", borderCls, streamActive && "tool-active")}>
        <button onClick={() => setOpen(o => !o)}
          className="w-full flex items-center gap-2 px-3 py-2 text-xs font-mono text-left hover:bg-white/[0.03] transition-colors">
          <Wrench className={cn("h-3 w-3 shrink-0", colorCls)} />
          <span className={cn("font-bold", colorCls)}>{tool}</span>
          {detailPreview && (
            <span className="text-muted-foreground/50 truncate ml-1 max-w-[140px]">{detailPreview}</span>
          )}
          <ChevronRight className={cn("h-3 w-3 text-muted-foreground/30 ml-auto shrink-0 transition-transform", open && "rotate-90")} />
        </button>
        {open && (
          <pre className="px-3 pb-2 text-[10px] text-muted-foreground/60 font-mono whitespace-pre-wrap border-t border-white/[0.05] max-h-40 overflow-y-auto">
            {JSON.stringify(event.input, null, 2)}
          </pre>
        )}
      </div>
    );
  }

  if (event.type === "tool_result") {
    const res = event.result as any;
    const isError = res?.error;
    const preview = res?.output ?? res?.content ?? res?.tree ?? res?.files
      ?? (typeof res === "string" ? res : JSON.stringify(res));
    return (
      <div className={cn(
        "flex items-start gap-2 px-3 py-2 rounded-xl border text-xs font-mono animate-fade-in",
        isError
          ? "border-red-200 bg-red-50 text-red-600"
          : "border-neutral-300 bg-neutral-100 text-neutral-700"
      )}>
        <CheckCircle2 className="h-3 w-3 shrink-0 mt-0.5" />
        <span className="text-muted-foreground/70 truncate leading-relaxed line-clamp-2">
          {String(preview ?? "ok").slice(0, 120)}
        </span>
      </div>
    );
  }
  return null;
}

// ── Mini file tree inside workspace selector ──────────────────────────────────
function MiniTree({ nodes, depth = 0 }: { nodes: TreeNode[]; depth?: number }) {
  return (
    <>
      {nodes.map(n => (
        <div key={n.path} style={{ paddingLeft: `${depth * 12}px` }}>
          <div className="flex items-center gap-1.5 py-0.5 text-[10px] text-muted-foreground/70 font-mono">
            {n.type === "dir"
              ? <FolderClosed className="h-3 w-3 text-neutral-400 shrink-0" />
              : <FileCode2  className="h-3 w-3 text-neutral-400 shrink-0" />}
            <span className="truncate">{n.name}</span>
          </div>
          {n.type === "dir" && n.children && (
            <MiniTree nodes={n.children} depth={depth + 1} />
          )}
        </div>
      ))}
    </>
  );
}

// ── Agent event stream — collapsible per-turn tool trace ─────────────────────
function AgentEventStream({ events, isStreaming: streamActive = false }: { events: AgentChatEvent[]; isStreaming?: boolean }) {
  const toolCount = events.filter(e => e.type === "tool_use").length;
  // Default expanded (not collapsed) when there are more than 3 events
  const [collapsed, setCollapsed] = useState(events.length <= 3);
  return (
    <div className="rounded-xl border border-neutral-200 bg-neutral-50 overflow-hidden animate-fade-in">
      <button
        onClick={() => setCollapsed(c => !c)}
        className="w-full flex items-center gap-2 px-3 py-2 text-xs text-left hover:bg-white/[0.02] transition-colors"
      >
        <BrainCircuit className={cn("h-3 w-3 text-neutral-700 shrink-0", streamActive && "animate-pulse")} />
        <span className="text-neutral-700 font-medium">الوكيل يفكّر</span>
        {toolCount > 0 && (
          <span className="ml-1 px-1.5 py-0.5 rounded-full bg-neutral-200 text-neutral-500 text-[9px] font-mono">
            {toolCount} tool{toolCount !== 1 ? "s" : ""}
          </span>
        )}
        {!streamActive && (
          <span className="ml-1 px-1.5 py-0.5 rounded-full bg-neutral-200 text-neutral-500 text-[9px] font-mono">
            تم
          </span>
        )}
        <ChevronRight className={cn("h-3 w-3 text-muted-foreground/30 ml-auto transition-transform", !collapsed && "rotate-90")} />
      </button>
      {!collapsed && (
        <div className="px-3 pb-2 space-y-1.5 max-h-48 overflow-y-auto">
          {events
        .filter((ev, i, arr) => {
          // For "thinking" type, only show the last one (deduplicate)
          if (ev.type === "thinking") {
            return arr.findLastIndex(e => e.type === "thinking") === i;
          }
          return true;
        })
        .map((ev, i) => <ToolUseCard key={i} event={ev} isStreaming={streamActive} />)}
        </div>
      )}
    </div>
  );
}

// ── Workspace selector pill ───────────────────────────────────────────────────
function WorkspaceSelector({
  value, onChange,
}: {
  value: string | null;
  onChange: (id: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const { data: workspaces = [], isLoading } = useWorkspaces();
  const { data: tree } = useWorkspaceTree(value ?? "");
  const current = workspaces.find(w => w.id === value);

  return (
    <div className="relative" onClick={e => e.stopPropagation()}>
      <button
        onClick={() => setOpen(o => !o)}
        className={cn(
          "flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs border transition-all",
          value
            ? "bg-neutral-200 border-neutral-300 text-neutral-700"
            : "bg-card/40 border-border/50 text-muted-foreground hover:text-foreground",
        )}
      >
        {value ? <FolderOpen className="h-3 w-3" /> : <FolderClosed className="h-3 w-3" />}
        <span>{current?.name ?? "مساحة العمل"}</span>
        <ChevronDown className="h-2.5 w-2.5" />
      </button>

      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute top-full left-0 mt-1.5 bg-card border border-border rounded-xl shadow-xl z-50 w-64 animate-scale-in overflow-hidden">
            {/* None option */}
            <button
              onClick={() => { onChange(null); setOpen(false); }}
              className={cn(
                "w-full flex items-center gap-2 px-3 py-2.5 text-xs transition-all hover:bg-white/[0.04]",
                !value ? "text-primary" : "text-muted-foreground",
              )}
            >
              <FolderClosed className="h-3.5 w-3.5 shrink-0" />
              <span>بدون مساحة عمل (محادثة أساسية)</span>
            </button>

            {isLoading ? (
              <div className="px-3 py-4 text-center text-xs text-muted-foreground/50">
                <Loader2 className="h-4 w-4 animate-spin mx-auto mb-1" />
              </div>
            ) : workspaces.length === 0 ? (
              <p className="px-3 py-3 text-xs text-muted-foreground/50 text-center">
                لا توجد مساحات عمل — أنشئ واحدة من تبويب المشاريع.
              </p>
            ) : (
              workspaces.map(w => (
                <button
                  key={w.id}
                  onClick={() => { onChange(w.id); setOpen(false); }}
                  className={cn(
                    "w-full flex items-start gap-2 px-3 py-2.5 text-xs transition-all hover:bg-white/[0.04] text-left border-t border-border/30",
                    value === w.id ? "bg-primary/8 text-primary" : "text-muted-foreground",
                  )}
                >
                  <FolderOpen className="h-3.5 w-3.5 shrink-0 mt-0.5 text-neutral-500" />
                  <div className="min-w-0">
                    <div className="font-medium truncate">{w.name}</div>
                    <div className="text-[10px] text-muted-foreground/50 mt-0.5">
                      {w.file_count} ملفات
                    </div>
                  </div>
                </button>
              ))
            )}

            {/* Tree preview for selected */}
            {value && tree && tree.tree.length > 0 && (
              <div className="border-t border-border/40 px-3 py-2 max-h-36 overflow-y-auto">
                <MiniTree nodes={tree.tree} />
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

// ── Mode metadata ─────────────────────────────────────────────────────────────
const MODE_META: Record<string, { Icon: React.ElementType; label: string; color: string; desc: string }> = {
  orchestrator: { Icon: Settings2, label: "منسّق", color: "text-primary",      desc: "ينسّق بين جميع الوكلاء · الأفضل للمهام الشاملة" },
  coder:        { Icon: Code2,     label: "مبرمج",        color: "text-neutral-600",     desc: "توليد الكود وإعادة الهيكلة والتعديلات" },
  planner:      { Icon: Command,   label: "مخطط",      color: "text-neutral-700",   desc: "البنية والتخطيط وتفكيك المهام" },
  debugger:     { Icon: Bug,       label: "مصحّح",     color: "text-red-600",     desc: "تحليل الأسباب الجذرية وتتبع الأخطاء" },
  designer:     { Icon: Palette,   label: "مصمم",     color: "text-neutral-500",     desc: "واجهات المستخدم والمكونات وأنظمة التصميم" },
  researcher:   { Icon: Search,    label: "باحث",   color: "text-neutral-500",    desc: "بحث معمّق وشرح المفاهيم" },
  explorer:     { Icon: Map,       label: "مستكشف",     color: "text-neutral-700",  desc: "استكشاف قاعدة الكود ورسم خريطة التبعيات" },
  security:     { Icon: Shield,    label: "أمان",     color: "text-neutral-600",   desc: "تدقيق الثغرات ومراجعة المصادقة" },
};

const EFFORT_META: Record<string, { label: string; color: string; steps: string; desc: string }> = {
  lite:   { label: "خفيف", color: "text-muted-foreground", steps: "3 خطوات",  desc: "إجابة سريعة · الأفضل للأسئلة البسيطة" },
  medium: { label: "متوسط",  color: "text-neutral-600",         steps: "7 خطوات",  desc: "متوازن · الأفضل لمعظم المهام"         },
  high:   { label: "عالي", color: "text-neutral-500",        steps: "12 خطوة", desc: "تحليل عميق · الأفضل للكود المعقد"  },
  max:    { label: "أقصى",  color: "text-red-600",         steps: "20 خطوة", desc: "العمك الكامل · الأفضل للبنية"     },
};

// ── Live agent status (replaces ThinkingIndicator) ────────────────────────────
function LiveAgentStatus({
  mode,
  events,
  lastThinking,
}: {
  mode: string;
  events: AgentChatEvent[];
  lastThinking: string;
}) {
  const m = MODE_META[mode] || MODE_META.orchestrator;
  const toolEvents = events.filter(e => e.type === "tool_use");
  const toolCount = toolEvents.length;
  const progressEvent = events.filter(e => e.type === "progress").at(-1) as any;
  const progress = progressEvent ? progressEvent.step / progressEvent.total : null;

  return (
      <div className="space-y-2 animate-fade-in">
      {/* Main status line */}
      <div className="flex items-center gap-2.5 py-2 px-3 rounded-xl bg-neutral-50 border border-neutral-200">
        <div className={cn(
          "h-6 w-6 rounded-lg flex items-center justify-center shrink-0",
          "bg-neutral-200"
        )}>
          <BrainCircuit className={cn("h-3.5 w-3.5 animate-pulse", m.color)} />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-xs text-foreground/80 font-medium truncate">
              {lastThinking || `${m.label} يعمل…`}
            </span>
            <span className="flex gap-0.5 shrink-0">
              {[0, 1, 2].map(i => (
                <span key={i} className="typing-dot h-1 w-1 rounded-full bg-neutral-400" />
              ))}
            </span>
          </div>
          {progress !== null && (
            <div className="mt-1.5 h-0.5 bg-border/40 rounded-full overflow-hidden">
              <div
                className="h-full bg-neutral-400 rounded-full transition-all duration-500"
                style={{ width: `${Math.round(progress * 100)}%` }}
              />
            </div>
          )}
        </div>
      </div>

      {/* Recent tool chips — last 4 */}
      {toolCount > 0 && (
        <div className="flex flex-wrap gap-1 px-1">
          {toolEvents.slice(-4).map((e: any, i) => {
            const tool = e.tool ?? "";
            const detail = e.input?.path ?? e.input?.command ?? e.input?.pattern ?? "";
            return (
              <div key={i} className="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono bg-neutral-100 border border-neutral-300 text-neutral-600">
                <Wrench className="h-2.5 w-2.5 shrink-0" />
                <span className="truncate max-w-[80px]">{tool}</span>
                {detail && (
                  <span className="text-neutral-500/50 truncate max-w-[60px]">{String(detail)}</span>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── Message bubble ────────────────────────────────────────────────────────────
function MessageBubble({ message, isNew }: {
  message: { id: string; role: string; content: string; modelUsed?: string | null; codeChanges?: string | null };
  isNew: boolean;
}) {
  if (message.role === "thinking") return (
    <div className={cn("flex justify-start", isNew && "animate-slide-up")}>
      <div className="bg-card/40 border border-border/40 rounded-xl px-3.5 py-2.5 max-w-[85%]">
        <div className="flex items-center gap-2 mb-1.5">
          <BrainCircuit className="h-3 w-3 text-neutral-700" />
          <span className="text-[10px] font-mono text-neutral-700 uppercase tracking-wider">thinking</span>
        </div>
        <p className="text-xs text-muted-foreground/70 italic leading-relaxed line-clamp-4">{message.content}</p>
      </div>
    </div>
  );

  if (message.role === "tool") return (
    <div className={cn("flex justify-start", isNew && "animate-slide-up")}>
      <div className="bg-neutral-950 border border-neutral-300 rounded-lg px-3 py-2 flex items-center gap-3 text-xs max-w-[85%]">
        <Wrench className="h-3 w-3 text-neutral-600 shrink-0" />
        <span className="font-mono text-neutral-600 truncate">{message.content.substring(0, 80)}</span>
        <CheckCircle2 className="h-3 w-3 text-neutral-700 ml-auto shrink-0" />
      </div>
    </div>
  );

  if (message.role === "user") return (
    <div className={cn("flex justify-end", isNew && "animate-slide-up")}>
      <div className="msg-user border rounded-2xl rounded-br-sm px-4 py-2.5 max-w-[85%] text-sm leading-relaxed">
        {message.content}
      </div>
    </div>
  );

  // Assistant — extract clean text from any JSON-wrapped content
  const displayContent = cleanDisplayText(message.content || "");

  return (
    <div className={cn("flex justify-start", isNew && "animate-slide-up")}>
      <div className="w-full">
        <div className="flex items-center gap-1.5 mb-1.5">
          <div className="h-4 w-4 rounded bg-primary/10 flex items-center justify-center">
            <Bot className="h-2.5 w-2.5 text-primary" />
          </div>
          <span className="text-[10px] text-muted-foreground/50 font-mono">PopCorn AI Studio</span>
          {/* Model name intentionally hidden — brand is always "PopCorn AI Studio" */}
        </div>
        <div className="msg-assistant border rounded-2xl rounded-tl-sm px-4 py-3 text-sm leading-relaxed message-content">
          <FormattedMessage content={displayContent} />
        </div>
        {message.codeChanges && (
          <div className="mt-2 bg-neutral-950 border border-border rounded-xl overflow-hidden">
            <div className="bg-neutral-900 px-3 py-2 border-b border-border flex items-center text-xs font-mono gap-2">
              <Code2 className="h-3 w-3 text-primary" />
              <span className="text-muted-foreground">تعديلات الملفات</span>
            </div>
            <pre className="p-3 text-xs font-mono overflow-x-auto">
              <code>
                {message.codeChanges.split("\n").map((line: string, i: number) => {
                  if (line.startsWith("+")) return <div key={i} className="text-neutral-700 bg-neutral-200 px-1">{line}</div>;
                  if (line.startsWith("-")) return <div key={i} className="text-red-600 bg-red-100 px-1">{line}</div>;
                  if (line.startsWith("@@")) return <div key={i} className="text-neutral-500 my-1">{line}</div>;
                  return <div key={i} className="text-muted-foreground px-1">{line}</div>;
                })}
              </code>
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

// ── Empty chat state ──────────────────────────────────────────────────────────
const SUGGESTIONS: Record<string, string[]> = {
  coder:        ["اكتب وحدات اختبار شاملة", "أعد هيكلة هذا الملف"],
  planner:      ["خطط بنية Microservices", "صمم خطة تطوير لمدة أسبوع"],
  debugger:     ["حلّل هذا الخطأ واقترح حلولاً", "تتبع مصدر تسريب الذاكرة"],
  researcher:   ["اشرح كيف يعمل React Query", "قارن بين أنماط التصميم"],
  designer:     ["صمم واجهة مستخدم حديثة", "أنشئ مكونات UI قابلة لإعادة الاستخدام"],
  explorer:     ["رسم خريطة تبعيات المشروع", "استكشف بنية المجلدات"],
  security:     ["فحص الثغرات الأمنية", "مراجعة تدفق المصادقة"],
  orchestrator: ["أنشئ مشروع React كامل مع باك إند", "نمّط قاعدة كود موجودة"],
};

function EmptyChat({ mode, onPrompt }: { mode: string; onPrompt: (t: string) => void }) {
  const m = MODE_META[mode] || MODE_META.orchestrator;
  const tips = SUGGESTIONS[mode] || SUGGESTIONS.orchestrator;
  return (
    <div className="flex flex-col items-center justify-center h-full px-6 text-center gap-5 animate-fade-in">
      <div className="relative">
        <div className={cn("h-16 w-16 rounded-2xl border flex items-center justify-center bg-card/60 shadow-lg", m.color.replace("text-", "border-").replace("400", "400/30"))}>
          <m.Icon className={cn("h-8 w-8", m.color)} />
        </div>
        <div className="absolute -bottom-1 -right-1 h-5 w-5 rounded-full bg-background border border-border flex items-center justify-center">
          <Sparkles className="h-2.5 w-2.5 text-primary" />
        </div>
      </div>
      <div className="space-y-1.5">
        <h2 className="text-base font-semibold tracking-tight">{m.label} <span className="gradient-text">جاهز</span></h2>
        <p className="text-xs text-muted-foreground/60 max-w-56 leading-relaxed">
          بوب كورن ستوديو جاهز. ابدأ محادثة أو اختر اقتراح.
        </p>
      </div>
      <div className="flex flex-col gap-2 w-full max-w-xs">
        {tips.map((tip, i) => (
          <button key={i} onClick={() => onPrompt(tip)}
            className="text-left text-xs px-3.5 py-2.5 rounded-xl border border-border/60 bg-card/30 hover:bg-card hover:border-primary/30 transition-all text-muted-foreground hover:text-foreground">
            {tip}
          </button>
        ))}
      </div>
    </div>
  );
}

// ── Session tab ───────────────────────────────────────────────────────────────
function SessionTab({ session, isActive, onSelect, onDelete, onRename }: {
  session: any; isActive: boolean;
  onSelect: () => void; onDelete: (e: React.MouseEvent) => void;
  onRename: (id: string, name: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(session.name);
  const inputRef = useRef<HTMLInputElement>(null);
  const m = MODE_META[session.mode] || MODE_META.orchestrator;
  useEffect(() => { if (editing) inputRef.current?.focus(); }, [editing]);

  function commit() {
    setEditing(false);
    if (name.trim()) onRename(session.id, name.trim());
    else setName(session.name);
  }

  return (
    <button onClick={onSelect}
      className={cn(
        "flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all shrink-0 group relative",
        isActive
          ? "bg-primary/12 text-primary border border-primary/25"
          : "text-muted-foreground hover:text-foreground hover:bg-white/[0.04] border border-transparent"
      )}>
      <m.Icon className={cn("h-3 w-3 shrink-0", isActive ? m.color : "text-muted-foreground")} />
      {editing ? (
        <input ref={inputRef} value={name} onChange={e => setName(e.target.value)}
          onBlur={commit} onClick={e => e.stopPropagation()}
          onKeyDown={e => { if (e.key === "Enter") commit(); if (e.key === "Escape") { setEditing(false); setName(session.name); } }}
          className="bg-transparent border-none outline-none text-xs w-20 text-foreground" />
      ) : (
        <span className="truncate max-w-[80px]" onDoubleClick={e => { e.stopPropagation(); setEditing(true); }}>
          {session.name}
        </span>
      )}
      <button onClick={onDelete}
        className={cn("ml-0.5 p-0.5 rounded opacity-0 group-hover:opacity-100 transition-opacity hover:text-destructive shrink-0", isActive && "opacity-60")}>
        <X className="h-2.5 w-2.5" />
      </button>
    </button>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────
export default function WorkspacePage() {
  const { data: sessions = [], isLoading: sessionsLoading } = useSessions();
  const { create, update, remove, isCreating } = useSessionMutations();
  const { toast } = useToast();
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [showModePanel, setShowModePanel] = useState(false);
  const [showEffortPanel, setShowEffortPanel] = useState(false);
  // ── Workspace context for agent tool-use ───────────────────────────────────
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(null);

  // Set initial session once loaded
  useEffect(() => {
    if (!sessionsLoading && sessions.length > 0 && !activeSessionId) {
      setActiveSessionId(sessions[0].id);
    }
  }, [sessions, sessionsLoading]);

  useEffect(() => {
    if (activeSessionId && sessions.length > 0 && !sessions.find(s => s.id === activeSessionId)) {
      setActiveSessionId(sessions[0]?.id ?? null);
    }
  }, [sessions]);

  const activeSession = sessions.find(s => s.id === activeSessionId);

  // Close dropdowns on outside click
  useEffect(() => {
    if (!showModePanel && !showEffortPanel) return;
    const h = () => { setShowModePanel(false); setShowEffortPanel(false); };
    document.addEventListener("click", h);
    return () => document.removeEventListener("click", h);
  }, [showModePanel, showEffortPanel]);

  async function handleCreateSession() {
    if (sessions.length >= 3) {
      toast({ title: "تم الوصول للحد", description: "الحد الأقصى 3 جلسات.", variant: "destructive" });
      return;
    }
    try {
      const s = await create({ name: `Session ${sessions.length + 1}`, mode: SessionMode.coder, effort: SessionEffort.medium });
      setActiveSessionId(s.id);
    } catch {
      toast({ title: "خطأ", description: "فشل إنشاء الجلسة.", variant: "destructive" });
    }
  }

  async function handleDeleteSession(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    try {
      await remove(id);
      if (activeSessionId === id) setActiveSessionId(null);
    } catch {
      toast({ title: "خطأ", description: "فشل حذف الجلسة.", variant: "destructive" });
    }
  }

  async function handleRenameSession(id: string, name: string) {
    try { await update(id, { name }); } catch {}
  }

  async function handleChangeMode(mode: SessionMode) {
    if (!activeSession) return;
    try { await update(activeSession.id, { mode }); setShowModePanel(false); } catch {}
  }

  async function handleChangeEffort(effort: SessionEffort) {
    if (!activeSession) return;
    try { await update(activeSession.id, { effort }); setShowEffortPanel(false); } catch {}
  }

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-hidden">

        {/* ── Session tabs bar ── */}
        <div className="shrink-0 flex items-center gap-2 px-3 pt-2 pb-1.5 border-b border-border/50 overflow-x-auto" style={{ scrollbarWidth: "none" }}>
          {sessionsLoading
            ? <div className="h-7 w-24 rounded-lg animate-shimmer" />
            : <>
                {sessions.map(s => (
                  <SessionTab key={s.id} session={s}
                    isActive={s.id === activeSessionId}
                    onSelect={() => setActiveSessionId(s.id)}
                    onDelete={e => handleDeleteSession(s.id, e)}
                    onRename={handleRenameSession} />
                ))}
                {sessions.length < 3 && (
                  <button onClick={handleCreateSession} disabled={isCreating}
                    className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs text-muted-foreground hover:text-foreground border border-dashed border-border/50 hover:border-border transition-all shrink-0">
                    {isCreating ? <Loader2 className="h-3 w-3 animate-spin" /> : <Plus className="h-3 w-3" />}
                    <span>جديد</span>
                  </button>
                )}
              </>
          }
        </div>

        {/* ── Mode / Effort toolbar ── */}
        {activeSession && (
          <div className="shrink-0 flex items-center gap-2 px-3 py-1.5 border-b border-border/30 relative z-30">
            {/* Mode */}
            <div className="relative" onClick={e => e.stopPropagation()}>
              <button onClick={() => { setShowModePanel(p => !p); setShowEffortPanel(false); }}
                className={cn("flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs border transition-all",
                  showModePanel ? "bg-primary/10 border-primary/30 text-primary" : "bg-card/40 border-border/50 text-muted-foreground hover:text-foreground")}>
                {(() => { const m = MODE_META[activeSession.mode] || MODE_META.orchestrator; return <m.Icon className={cn("h-3 w-3", m.color)} />; })()}
                <span>{MODE_META[activeSession.mode]?.label || activeSession.mode}</span>
                <ChevronDown className="h-2.5 w-2.5" />
              </button>
              {showModePanel && (
                <>
                  {/* Backdrop to close dropdown when clicking outside */}
                  <div className="fixed inset-0 z-40" onClick={() => setShowModePanel(false)} />
                  <div className="absolute top-full left-0 mt-1.5 bg-card border border-border rounded-xl shadow-xl z-50 p-1.5 flex flex-col gap-0.5 w-64 animate-scale-in">
                    {Object.entries(MODE_META).map(([key, { Icon, label, color, desc }]) => (
                      <button key={key} onClick={() => handleChangeMode(key as SessionMode)}
                        className={cn("flex items-start gap-2.5 px-2.5 py-2 rounded-lg text-xs transition-all text-left",
                          activeSession.mode === key ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground")}>
                        <Icon className={cn("h-3 w-3 shrink-0 mt-0.5", color)} />
                        <div className="min-w-0">
                          <div className="font-medium leading-tight">{label}</div>
                          <div className="text-[10px] text-muted-foreground/50 mt-0.5 leading-snug">{desc}</div>
                        </div>
                      </button>
                    ))}
                  </div>
                </>
              )}
            </div>

            {/* Effort */}
            <div className="relative" onClick={e => e.stopPropagation()}>
              <button onClick={() => { setShowEffortPanel(p => !p); setShowModePanel(false); }}
                className={cn("flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs border transition-all",
                  showEffortPanel ? "bg-primary/10 border-primary/30 text-primary" : "bg-card/40 border-border/50 text-muted-foreground hover:text-foreground")}>
                <Zap className={cn("h-3 w-3", EFFORT_META[activeSession.effort]?.color)} />
                <span>{EFFORT_META[activeSession.effort]?.label || activeSession.effort}</span>
                <ChevronDown className="h-2.5 w-2.5" />
              </button>
              {showEffortPanel && (
                <>
                  <div className="fixed inset-0 z-40" onClick={() => setShowEffortPanel(false)} />
                  <div className="absolute top-full left-0 mt-1.5 bg-card border border-border rounded-xl shadow-xl z-50 p-1.5 w-56 animate-scale-in">
                    {Object.entries(EFFORT_META).map(([key, { label, color, steps, desc }]) => (
                      <button key={key} onClick={() => handleChangeEffort(key as SessionEffort)}
                        className={cn("flex items-start gap-2.5 w-full px-2.5 py-2 rounded-lg text-xs transition-all text-left",
                          activeSession.effort === key ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground")}>
                        <Zap className={cn("h-3 w-3 shrink-0 mt-0.5", color)} />
                        <div className="min-w-0">
                          <div className="flex items-center gap-1.5">
                            <span className={cn("font-semibold", color)}>{label}</span>
                            <span className="text-[10px] text-muted-foreground/50 font-mono">{steps}</span>
                          </div>
                          <div className="text-[10px] text-muted-foreground/50 mt-0.5 leading-snug">{desc}</div>
                        </div>
                      </button>
                    ))}
                  </div>
                </>
              )}
            </div>

            {/* Workspace selector */}
            <WorkspaceSelector
              value={activeWorkspaceId}
              onChange={setActiveWorkspaceId}
            />

            <div className="ml-auto flex items-center gap-1.5 text-[10px] text-muted-foreground/40 font-mono">
              <span className="h-1.5 w-1.5 rounded-full bg-neutral-400" />
              متصل
            </div>
          </div>
        )}

        {/* ── Chat or welcome ── */}
        {activeSessionId ? (
          <ChatPanel
            key={activeSessionId}
            sessionId={activeSessionId}
            mode={activeSession?.mode || SessionMode.orchestrator}
            effort={activeSession?.effort || SessionEffort.medium}
            workspaceId={activeWorkspaceId ?? undefined}
          />
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center gap-5 px-6 text-center animate-fade-in">
            <div className="h-14 w-14 rounded-2xl bg-primary/8 border border-primary/15 flex items-center justify-center animate-float">
              <Terminal className="h-7 w-7 text-primary" />
            </div>
            <div className="space-y-1.5">
              <h1 className="text-lg font-semibold gradient-text">PopCorn AI Studio</h1>
              <p className="text-xs text-muted-foreground/60 max-w-52">أنشئ جلسة للبدء.</p>
            </div>
            <button onClick={handleCreateSession} disabled={isCreating}
              className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-lg shadow-primary/20">
              {isCreating ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
              جلسة جديدة
            </button>
          </div>
        )}
      </div>
    </AppLayout>
  );
}

// ── Chat Panel ────────────────────────────────────────────────────────────────
// Key is passed from parent (key={activeSessionId}) so this remounts per session
function ChatPanel({ sessionId, mode, effort, workspaceId }: {
  sessionId: string; mode: string; effort: string; workspaceId?: string;
}) {
  const { data: messages = [], isLoading } = useMessages(sessionId);
  const { add: addMessage, invalidateMessages } = useMessageMutations(sessionId);
  const { toast } = useToast();

  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  // Optimistic user message — shown immediately before DB save completes
  const [optimisticUserMsg, setOptimisticUserMsg] = useState<{id: string; content: string} | null>(null);
  const [streamContent, setStreamContent] = useState("");
  const [streamThinking, setStreamThinking] = useState("");
  const [lastThinking, setLastThinking] = useState("");
  // Attached images for vision analysis
  const [attachedImages, setAttachedImages] = useState<Array<{url: string; name: string}>>([]);
  // Tool-use events during agent loop — persist after streaming ends
  const [agentEvents, setAgentEvents] = useState<AgentChatEvent[]>([]);
  // Last completed stream content — held until confirmed in messages[] (prevents flicker)
  const [pendingMessage, setPendingMessage] = useState<string | null>(null);
  const [newIds, setNewIds] = useState<Set<string>>(new Set());

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef    = useRef<HTMLTextAreaElement>(null);
  const abortRef       = useRef<AbortController | null>(null);

  // Auto-scroll on new content
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamContent]);

  // Auto-resize textarea
  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = Math.min(ta.scrollHeight, 140) + "px";
  }, [input]);

  const modelId = ROLE_MODEL_MAP[mode] || "deepseek-v4-flash-free";

  async function handleSend() {
    const text = input.trim();
    const hasImages = attachedImages.length > 0;
    if (!text && !hasImages) return;
    if (isStreaming) return;

    setInput("");
    const imagesToSend = [...attachedImages];
    setAttachedImages([]);
    setStreamContent("");
    setStreamThinking("");
    setLastThinking("");
    setAgentEvents([]);
    setPendingMessage(null);

    // Build display content for user message
    const userDisplayContent = hasImages
      ? `${text || ""}${text ? "\n" : ""}[${imagesToSend.length} صورة${imagesToSend.length > 1 ? "ات" : ""} مرفقة]`
      : text;

    // ── OPTIMISTIC UPDATE: show user message IMMEDIATELY before any async work ──
    const optimisticId = `opt_${Date.now()}`;
    setOptimisticUserMsg({ id: optimisticId, content: userDisplayContent || "[image]" });
    setIsStreaming(true);

    try {
      // 1. Save user message to DB (fire & don't block UI — optimistic already shown)
      // Save user message. Use skipInvalidate=false so the cache refreshes
      // immediately — this makes the real message replace the optimistic one atomically.
      const userMsg = await addMessage({ role: "user", content: userDisplayContent || text || "[image]" }, false);
      setNewIds(prev => new Set([...prev, userMsg.id]));
      // Now that messages[] has been invalidated and will refetch, clear optimistic.
      // The real message will appear from the refetch; no gap.
      setOptimisticUserMsg(null);

      abortRef.current = new AbortController();
      let cleanFull = "";

      // ── Path A: Workspace agent loop (tool-use) ───────────────────────────
      if (workspaceId) {
        // Build history for context continuity (last 8 turns)
        const history = messages
          .slice(-8)
          .filter((m: any) => m.role === "user" || m.role === "assistant")
          .map((m: any) => ({ role: m.role, content: m.content }));

        for await (const event of streamAgentChat(
          text || (hasImages ? "حلّل هذه الصورة" : ""), workspaceId, sessionId, mode, effort, history, abortRef.current.signal,
          imagesToSend.map(img => ({ url: img.url }))
        )) {
          if (event.type === "thinking") {
            // Replace last thinking event instead of accumulating
            setAgentEvents(prev => {
              const without = prev.filter(e => e.type !== "thinking");
              return [...without, event];
            });
            setLastThinking(event.content ?? "");
          } else if (
            event.type === "tool_use" ||
            event.type === "tool_result" ||
            event.type === "memory_hit"
          ) {
            setAgentEvents(prev => [...prev, event]);
          } else if (event.type === "progress" || event.type === "file_written") {
            setAgentEvents(prev => [...prev, event]);
          } else if (event.type === "text") {
            const rawContent = event.content ?? "";
            // The backend always sends clean text in event.content — no JSON guard needed.
            // If somehow a raw JSON string leaks, cleanDisplayText will unwrap it.
            const clean = cleanDisplayText(rawContent);
            if (clean) {
              cleanFull = clean;
              setStreamContent(cleanFull);
            }
          } else if (event.type === "error") {
            throw new Error(event.message ?? "خطأ في الوكيل");
          }
        }
      }
      // ── Path B: Standard zen chat (no workspace) ──────────────────────────
      else {
        let systemPrompt = "You are PopCorn AI Studio — a powerful AI coding and research assistant. Be thorough, precise, and proactive.";
        try {
          const rag = await fetchRagContext(text, sessionId, 1200);
          if (rag?.systemContext) systemPrompt += `\n\nRelevant memory:\n${rag.systemContext}`;
        } catch { /* RAG optional */ }

        const history = messages
          .slice(-12)
          .filter((m: any) => m.role === "user" || m.role === "assistant")
          .map((m: any) => ({ role: m.role, content: m.content }));

        const apiMessages = [
          { role: "system", content: systemPrompt },
          ...history,
          { role: "user", content: text },
        ];

        let full = "";
        for await (const chunk of streamZenChat(modelId, apiMessages, abortRef.current.signal)) {
          // Backend already strips JSON from stream chunks via zen.rs buffer parser.
          // cleanDisplayText handles any remaining edge cases.
          const cleanChunk = cleanDisplayText(chunk);
          if (cleanChunk) {
            full += cleanChunk;
            setStreamContent(full);
          }
        }
        cleanFull = cleanDisplayText(full);
      }

      // 2. Streaming done — transition: hold content in pendingMessage, clear stream UI
      setIsStreaming(false);
      setStreamContent("");
      setStreamThinking("");
      setPendingMessage(cleanFull);

      // 3. Save assistant message — skip invalidate until AFTER we verify it's in the list
      const assistantMsg = await addMessage({
        role: "assistant",
        content: cleanFull,
        modelUsed: modelId,
      } as any, true);
      setNewIds(prev => new Set([...prev, assistantMsg.id]));

      // 4. NOW invalidate — message is persisted, re-fetch will include it
      invalidateMessages();
      // Clear pending only after invalidation is fired (messages will re-fetch)
      setPendingMessage(null);

      // 5. Auto-store to RAG
      autoStoreMemory(text, cleanFull, sessionId).catch(() => {});

    } catch (err: any) {
      if (err.name !== "AbortError") {
        toast({
          title: "خطأ في الوكيل",
          description: err.message || "فشل الاتصال بالخادم.",
          variant: "destructive",
        });
      }
      setIsStreaming(false);
      setStreamContent("");
      setStreamThinking("");
      setLastThinking("");
      setPendingMessage(null);
      setOptimisticUserMsg(null);
      // Invalidate so any partial saves show up
      invalidateMessages();
    }
  }

  function handleAbort() {
    abortRef.current?.abort();
    setIsStreaming(false);
    setStreamContent("");
    setStreamThinking("");
    setLastThinking("");
    setPendingMessage(null);
    setOptimisticUserMsg(null);
    invalidateMessages();
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handleSend(); }
  }

  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
      {/* Messages — flex-1 + min-h-0 prevents overlap with prompt */}
      <div className="flex-1 overflow-y-auto min-h-0 px-3 py-3 space-y-3 chat-scroll-area">
        {isLoading ? (
          <div className="flex items-center justify-center py-10">
            <Loader2 className="h-5 w-5 animate-spin text-primary/60" />
          </div>
        ) : messages.length === 0 && !isStreaming ? (
          <EmptyChat mode={mode} onPrompt={t => { setInput(t); textareaRef.current?.focus(); }} />
        ) : (
          <>
            {messages.map((m: any) => (
              <MessageBubble key={m.id} message={m} isNew={newIds.has(m.id)} />
            ))}

            {/* Optimistic user message — shown immediately, disappears once in messages[] */}
            {optimisticUserMsg && !messages.find((m: any) => m.content === optimisticUserMsg.content && m.role === "user") && (
              <div className="flex justify-end animate-fade-in">
                <div className="msg-user border rounded-2xl rounded-br-sm px-4 py-2.5 max-w-[85%] text-sm leading-relaxed">
                  {optimisticUserMsg.content}
                </div>
              </div>
            )}

            {/* Agent tool-use events — show while streaming AND persist after (collapsed "done" state) */}
            {agentEvents.length > 0 && (
              <AgentEventStream events={agentEvents} isStreaming={isStreaming} />
            )}

            {/* Streaming message — visible only while actively streaming */}
            {isStreaming && (
              <div className="flex justify-start animate-fade-in">
                <div className="w-full max-w-full space-y-2">
                  {/* Live agent status — always visible while working, before text arrives */}
                  {!streamContent && (
                    <LiveAgentStatus
                      mode={mode}
                      events={agentEvents}
                      lastThinking={lastThinking}
                    />
                  )}

                  {/* Streaming text output */}
                  {streamContent && (
                    <div className="msg-assistant border rounded-2xl rounded-tl-sm px-4 py-3 text-sm leading-relaxed message-content stream-container">
                      <div className="flex items-center gap-1.5 mb-1.5">
                        <div className="h-4 w-4 rounded bg-primary/10 flex items-center justify-center">
                          <Bot className="h-2.5 w-2.5 text-primary" />
                        </div>
                        <span className="text-[10px] text-muted-foreground/50 font-mono">PopCorn AI Studio</span>
                        {workspaceId && (
                          <span className="text-[9px] px-1.5 py-0.5 rounded bg-neutral-200 text-neutral-600 font-mono ml-1">
                            مساحة عمل
                          </span>
                        )}
                      </div>
                      <FormattedMessage content={streamContent} />
                      <span className="stream-cursor" />
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Pending message — bridges the gap between stream end and DB re-fetch */}
            {!isStreaming && pendingMessage &&
              !messages.find((m: any) => m.content === pendingMessage) && (
              <div className="flex justify-start msg-appear">
                <div className="w-full">
                  <div className="flex items-center gap-1.5 mb-1.5">
                    <div className="h-4 w-4 rounded bg-primary/10 flex items-center justify-center">
                      <Bot className="h-2.5 w-2.5 text-primary" />
                    </div>
                    <span className="text-[10px] text-muted-foreground/50 font-mono">PopCorn AI Studio</span>
                  </div>
                  <div className="msg-assistant border rounded-2xl rounded-tl-sm px-4 py-3 text-sm leading-relaxed message-content">
                    <FormattedMessage content={pendingMessage} />
                  </div>
                </div>
              </div>
            )}
          </>
        )}
        <div ref={messagesEndRef} className="h-1" />
      </div>

      {/* Prompt box */}
      <div className="shrink-0 px-3 pb-3 pt-2 border-t border-border/40">
        {/* Image preview strip */}
        {attachedImages.length > 0 && (
          <div className="flex gap-2 mb-2 flex-wrap px-1">
            {attachedImages.map((img: any, i: number) => (
              <div key={i} className="relative group h-14 w-14 rounded-lg overflow-hidden border border-border/60">
                <img src={img.url} alt="" className="h-full w-full object-cover" />
                <button
                  onClick={() => setAttachedImages((prev: any[]) => prev.filter((_: any, j: number) => j !== i))}
                  className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 flex items-center justify-center transition-opacity"
                >
                  <X className="h-3.5 w-3.5 text-white" />
                </button>
              </div>
            ))}
          </div>
        )}

        <div className={cn(
          "flex items-end gap-2 rounded-2xl border bg-card/60 px-3 py-2 transition-all duration-200",
          isStreaming ? "border-primary/30 bg-primary/[0.02]" : "border-border/60 focus-within:border-primary/40",
          attachedImages.length > 0 && "border-neutral-300"
        )}>
          {/* Image attach */}
          <label className="flex items-center pb-0.5 shrink-0 cursor-pointer" title="إرفاق صورة للتحليل">
            <div className="h-6 w-6 rounded-lg flex items-center justify-center text-muted-foreground/40 hover:text-neutral-700 hover:bg-neutral-100 transition-all">
              <Palette className="h-3.5 w-3.5" />
            </div>
            <input type="file" accept="image/*" multiple className="hidden"
              onChange={async (e) => {
                const files = Array.from(e.target.files ?? []);
                for (const f of files) {
                  const url = await new Promise<string>((res) => {
                    const reader = new FileReader();
                    reader.onload = () => res(reader.result as string);
                    reader.readAsDataURL(f);
                  });
                  setAttachedImages((prev: any[]) => [...prev, { url, name: f.name }]);
                }
                e.target.value = "";
              }}
            />
          </label>

          <Textarea
            ref={textareaRef}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={
              isStreaming ? "الوكيل يستجيب…" :
              attachedImages.length > 0 ? "اسأل عن الصورة…" :
              "اكتب رسالة…"
            }
            disabled={isStreaming}
            rows={1}
            className="flex-1 resize-none border-none bg-transparent p-0 text-sm placeholder:text-muted-foreground/35 focus-visible:ring-0 focus-visible:ring-offset-0 min-h-[24px] max-h-[140px] leading-relaxed disabled:opacity-50"
          />
          <div className="flex items-center pb-0.5 shrink-0">
            {isStreaming ? (
              <button onClick={handleAbort}
                className="h-7 w-7 rounded-lg bg-red-100 text-red-600 flex items-center justify-center hover:bg-red-200 transition-colors active:scale-90"
                title="إيقاف التوليد">
                <RotateCcw className="h-3.5 w-3.5" />
              </button>
            ) : (
              <button
                onClick={handleSend}
                disabled={!input.trim() && attachedImages.length === 0}
                className={cn(
                  "h-7 w-7 rounded-lg flex items-center justify-center transition-all",
                  (input.trim() || attachedImages.length > 0)
                    ? attachedImages.length > 0
                      ? "bg-black text-white hover:bg-neutral-800 shadow-md shadow-black/20 active:scale-90"
                      : "bg-primary text-primary-foreground hover:bg-primary/90 shadow-md shadow-primary/25 active:scale-90"
                    : "bg-muted text-muted-foreground/40 cursor-not-allowed"
                )}
                title="إرسال (Enter)"
              >
                <ArrowUp className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>
        <div className="flex items-center justify-between mt-1 px-1">
          <p className="text-[10px] text-muted-foreground/25 font-mono">
            Enter · Shift+Enter سطر جديد
          </p>
          {attachedImages.length > 0 && (
            <p className="text-[10px] text-neutral-400 font-mono animate-fade-in">
              {attachedImages.length} صورة{attachedImages.length > 1 ? "ات" : ""} · رؤية
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

