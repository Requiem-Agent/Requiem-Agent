import { useState, useRef, useEffect } from "react";
import { AppLayout } from "@/components/layout";
import {
  useSessions,
  useSessionMutations,
  useMessageMutations,
  useMessages,
} from "@/hooks/use-sessions";
import { ROLE_MODEL_MAP, FREE_ZEN_MODELS } from "@/hooks/use-system";
import { FormattedMessage } from "@/components/message-formatter";
import { streamZenChat, extractTextFromJson } from "@/lib/zen-client";
import { Textarea } from "@/components/ui/textarea";
import { SessionMode, SessionEffort } from "@workspace/api-client-react";
import { useToast } from "@/hooks/use-toast";
import {
  Terminal, Plus, X, Command, Code2, Settings2, Palette,
  Bug, Search, Map, Shield, Loader2, BrainCircuit,
  Wrench, CheckCircle2, ChevronDown, Zap, RotateCcw,
  Bot, Sparkles, ArrowUp,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { fetchRagContext, autoStoreMemory } from "@/hooks/use-memory";

// ── Mode metadata ─────────────────────────────────────────────────────────────
const MODE_META: Record<string, { Icon: React.ElementType; label: string; color: string; desc: string }> = {
  orchestrator: { Icon: Settings2, label: "Orchestrator", color: "text-primary",      desc: "Coordinates all models" },
  coder:        { Icon: Code2,     label: "Coder",        color: "text-cyan-400",     desc: "Code generation & edits" },
  planner:      { Icon: Command,   label: "Planner",      color: "text-violet-400",   desc: "Architectural planning" },
  debugger:     { Icon: Bug,       label: "Debugger",     color: "text-rose-400",     desc: "Root-cause analysis" },
  designer:     { Icon: Palette,   label: "Designer",     color: "text-pink-400",     desc: "UI/UX & creative" },
  researcher:   { Icon: Search,    label: "Researcher",   color: "text-amber-400",    desc: "Deep research" },
  explorer:     { Icon: Map,       label: "Explorer",     color: "text-emerald-400",  desc: "Codebase navigation" },
  security:     { Icon: Shield,    label: "Security",     color: "text-orange-400",   desc: "Vulnerability analysis" },
};

const EFFORT_META: Record<string, { label: string; color: string }> = {
  lite:   { label: "Lite",   color: "text-muted-foreground" },
  medium: { label: "Med",    color: "text-cyan-400"         },
  high:   { label: "High",   color: "text-amber-400"        },
  max:    { label: "Max",    color: "text-rose-400"         },
};

// ── Typing indicator ──────────────────────────────────────────────────────────
function ThinkingIndicator({ mode }: { mode: string }) {
  const m = MODE_META[mode] || MODE_META.orchestrator;
  const stages = ["Analyzing context…", "Retrieving memory…", `${m.label} active…`, "Generating…"];
  const [stage, setStage] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setStage(s => (s + 1) % stages.length), 1400);
    return () => clearInterval(t);
  }, []);
  return (
    <div className="flex items-center gap-3 py-2 animate-fade-in">
      <div className={cn("h-7 w-7 rounded-lg flex items-center justify-center bg-card border border-border/60 shrink-0", m.color)}>
        <m.Icon className="h-3.5 w-3.5" />
      </div>
      <span className="text-xs text-muted-foreground font-mono">{stages[stage]}</span>
      <span className="flex gap-0.5 mt-0.5">
        {[0, 1, 2].map(i => <span key={i} className="typing-dot h-1.5 w-1.5 rounded-full bg-primary/60" />)}
      </span>
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
          <BrainCircuit className="h-3 w-3 text-violet-400" />
          <span className="text-[10px] font-mono text-violet-400 uppercase tracking-wider">thinking</span>
        </div>
        <p className="text-xs text-muted-foreground/70 italic leading-relaxed line-clamp-4">{message.content}</p>
      </div>
    </div>
  );

  if (message.role === "tool") return (
    <div className={cn("flex justify-start", isNew && "animate-slide-up")}>
      <div className="bg-[#0a0c10] border border-cyan-500/20 rounded-lg px-3 py-2 flex items-center gap-3 text-xs max-w-[85%]">
        <Wrench className="h-3 w-3 text-cyan-500 shrink-0" />
        <span className="font-mono text-cyan-400 truncate">{message.content.substring(0, 80)}</span>
        <CheckCircle2 className="h-3 w-3 text-emerald-500 ml-auto shrink-0" />
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
  const rawContent = message.content || "";
  let displayContent = extractTextFromJson(rawContent) ?? rawContent;
  // Safety net: if displayContent still looks like JSON, try once more
  if (displayContent.trim().startsWith("{") || displayContent.trim().startsWith("[")) {
    const inner = extractTextFromJson(displayContent);
    if (inner) displayContent = inner;
  }

  return (
    <div className={cn("flex justify-start", isNew && "animate-slide-up")}>
      <div className="w-full">
        <div className="flex items-center gap-1.5 mb-1.5">
          <div className="h-4 w-4 rounded bg-primary/10 flex items-center justify-center">
            <Bot className="h-2.5 w-2.5 text-primary" />
          </div>
          <span className="text-[10px] text-muted-foreground/50 font-mono">Requiem Agent 1</span>
          {message.modelUsed && (
            <span className="text-[9px] text-muted-foreground/30 font-mono ml-1">· {message.modelUsed.split("/").pop()}</span>
          )}
        </div>
        <div className="msg-assistant border rounded-2xl rounded-tl-sm px-4 py-3 text-sm leading-relaxed message-content">
          <FormattedMessage content={displayContent} />
        </div>
        {message.codeChanges && (
          <div className="mt-2 bg-[#0a0c10] border border-border rounded-xl overflow-hidden">
            <div className="bg-[#0f1014] px-3 py-2 border-b border-border flex items-center text-xs font-mono gap-2">
              <Code2 className="h-3 w-3 text-primary" />
              <span className="text-muted-foreground">File Modifications</span>
            </div>
            <pre className="p-3 text-xs font-mono overflow-x-auto">
              <code>
                {message.codeChanges.split("\n").map((line: string, i: number) => {
                  if (line.startsWith("+")) return <div key={i} className="text-emerald-400 bg-emerald-400/10 px-1">{line}</div>;
                  if (line.startsWith("-")) return <div key={i} className="text-rose-400 bg-rose-400/10 px-1">{line}</div>;
                  if (line.startsWith("@@")) return <div key={i} className="text-cyan-400 my-1">{line}</div>;
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
  coder:        ["Write a REST API in TypeScript", "Create a React component", "Refactor this function"],
  planner:      ["Plan microservices architecture", "Break down this project", "Design a database schema"],
  debugger:     ["Debug this error trace", "Find memory leaks", "Trace this crash"],
  researcher:   ["Explain this concept deeply", "Compare these approaches", "Research best practices"],
  designer:     ["Design a landing page", "Create a UI component library", "Build a color system"],
  explorer:     ["Explore the codebase", "Find all API endpoints", "Map all dependencies"],
  security:     ["Scan for vulnerabilities", "Review auth flow", "Check for SQL injection"],
  orchestrator: ["Build a full-stack app", "Coordinate a complex task", "Multi-model analysis"],
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
        <h2 className="text-base font-semibold tracking-tight">{m.label} <span className="gradient-text">Ready</span></h2>
        <p className="text-xs text-muted-foreground/60 max-w-56 leading-relaxed">
          Requiem Agent 1 is listening. Start a conversation or pick a suggestion.
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
      toast({ title: "Limit reached", description: "Max 3 sessions.", variant: "destructive" });
      return;
    }
    try {
      const s = await create({ name: `Session ${sessions.length + 1}`, mode: SessionMode.coder, effort: SessionEffort.medium });
      setActiveSessionId(s.id);
    } catch {
      toast({ title: "Error", description: "Failed to create session.", variant: "destructive" });
    }
  }

  async function handleDeleteSession(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    try {
      await remove(id);
      if (activeSessionId === id) setActiveSessionId(null);
    } catch {
      toast({ title: "Error", description: "Failed to delete session.", variant: "destructive" });
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
                    <span>New</span>
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
                  <div className="absolute top-full left-0 mt-1.5 bg-card border border-border rounded-xl shadow-xl z-50 p-1.5 grid grid-cols-2 gap-0.5 w-56 animate-scale-in">
                    {Object.entries(MODE_META).map(([key, { Icon, label, color }]) => (
                      <button key={key} onClick={() => handleChangeMode(key as SessionMode)}
                        className={cn("flex items-center gap-2 px-2.5 py-2 rounded-lg text-xs transition-all",
                          activeSession.mode === key ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground")}>
                        <Icon className={cn("h-3 w-3 shrink-0", color)} />
                        <span className="font-medium">{label}</span>
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
                  <div className="absolute top-full left-0 mt-1.5 bg-card border border-border rounded-xl shadow-xl z-50 p-1.5 w-40 animate-scale-in">
                    {Object.entries(EFFORT_META).map(([key, { label, color }]) => (
                      <button key={key} onClick={() => handleChangeEffort(key as SessionEffort)}
                        className={cn("flex items-center justify-between w-full px-2.5 py-2 rounded-lg text-xs transition-all",
                          activeSession.effort === key ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground")}>
                        <span className={cn("font-medium", color)}>{label}</span>
                      </button>
                    ))}
                  </div>
                </>
              )}
            </div>

            <div className="ml-auto flex items-center gap-1.5 text-[10px] text-muted-foreground/40 font-mono">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-400/60" />
              online
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
          />
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center gap-5 px-6 text-center animate-fade-in">
            <div className="h-14 w-14 rounded-2xl bg-primary/8 border border-primary/15 flex items-center justify-center animate-float">
              <Terminal className="h-7 w-7 text-primary" />
            </div>
            <div className="space-y-1.5">
              <h1 className="text-lg font-semibold gradient-text">Requiem Agent 1</h1>
              <p className="text-xs text-muted-foreground/60 max-w-52">Create a session to start.</p>
            </div>
            <button onClick={handleCreateSession} disabled={isCreating}
              className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-lg shadow-primary/20">
              {isCreating ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
              New Session
            </button>
          </div>
        )}
      </div>
    </AppLayout>
  );
}

// ── Chat Panel ────────────────────────────────────────────────────────────────
// Key is passed from parent (key={activeSessionId}) so this remounts per session
function ChatPanel({ sessionId, mode, effort }: {
  sessionId: string; mode: string; effort: string;
}) {
  const { data: messages = [], isLoading } = useMessages(sessionId);
  // ← Correct: useMessageMutations takes sessionId, returns { add, isAdding }
  const { add: addMessage } = useMessageMutations(sessionId);
  const { toast } = useToast();

  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamContent, setStreamContent] = useState("");
  const [streamThinking, setStreamThinking] = useState("");
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
    if (!text || isStreaming) return;
    setInput("");
    setIsStreaming(true);
    setStreamContent("");
    setStreamThinking("");

    try {
      // 1. Save user message to backend (correct: add(data), not add(sessionId, data))
      const userMsg = await addMessage({ role: "user", content: text });
      setNewIds(prev => new Set([...prev, userMsg.id]));

      // 2. Fetch RAG context from backend
      let systemPrompt = "You are Requiem Agent 1 — a powerful AI coding and research assistant. Be thorough, precise, and proactive.";
      try {
        const rag = await fetchRagContext(text, sessionId, 1200);
        if (rag?.systemContext) {
          systemPrompt += `\n\nRelevant memory:\n${rag.systemContext}`;
        }
      } catch {
        // RAG optional — continue without it
      }

      // 3. Build message history (last 12 exchanges)
      const history = messages
        .slice(-12)
        .filter((m: any) => m.role === "user" || m.role === "assistant")
        .map((m: any) => ({ role: m.role, content: m.content }));

      const apiMessages = [
        { role: "system", content: systemPrompt },
        ...history,
        { role: "user", content: text },
      ];

      // 4. Stream from backend /api/zen/chat (goes through Rust/Axum → Zen AI via SOCKS5 proxies)
      abortRef.current = new AbortController();
      let full = "";

      for await (const chunk of streamZenChat(modelId, apiMessages, abortRef.current.signal)) {
        full += chunk;
        setStreamContent(full);
      }

      // Extract clean text if the full response ended up being JSON-wrapped
      const cleanFull = extractTextFromJson(full) ?? full;

      // 5. Save assistant message to backend
      const assistantMsg = await addMessage({
        role: "assistant",
        content: cleanFull,
        modelUsed: modelId,
      } as any);
      setNewIds(prev => new Set([...prev, assistantMsg.id]));

      // 6. Auto-store to RAG memory (fire & forget)
      autoStoreMemory(text, cleanFull, sessionId).catch(() => {});

    } catch (err: any) {
      if (err.name !== "AbortError") {
        toast({
          title: "Agent error",
          description: err.message || "Failed to reach backend.",
          variant: "destructive",
        });
      }
    } finally {
      setIsStreaming(false);
      setStreamContent("");
      setStreamThinking("");
    }
  }

  function handleAbort() {
    abortRef.current?.abort();
    setIsStreaming(false);
    setStreamContent("");
    setStreamThinking("");
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

            {/* Streaming message */}
            {isStreaming && (
              <div className="flex justify-start animate-fade-in">
                <div className="w-full max-w-full space-y-2">
                  {streamContent ? (
                    <div className="msg-assistant border rounded-2xl rounded-tl-sm px-4 py-3 text-sm leading-relaxed message-content stream-container">
                      <div className="flex items-center gap-1.5 mb-1.5">
                        <div className="h-4 w-4 rounded bg-primary/10 flex items-center justify-center">
                          <Bot className="h-2.5 w-2.5 text-primary" />
                        </div>
                        <span className="text-[10px] text-muted-foreground/50 font-mono">Requiem Agent 1</span>
                      </div>
                      <FormattedMessage content={streamContent} />
                      <span className="stream-cursor" />
                    </div>
                  ) : (
                    <ThinkingIndicator mode={mode} />
                  )}
                </div>
              </div>
            )}
          </>
        )}
        <div ref={messagesEndRef} className="h-1" />
      </div>

      {/* Prompt box — shrink-0 guarantees it never gets pushed up */}
      <div className="shrink-0 px-3 pb-3 pt-2 border-t border-border/40">
        <div className={cn(
          "flex items-end gap-2 rounded-2xl border bg-card/60 px-3 py-2 transition-all duration-200",
          isStreaming ? "border-primary/30" : "border-border/60 focus-within:border-primary/40"
        )}>
          <Textarea
            ref={textareaRef}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={isStreaming ? "Agent is responding…" : "Message Requiem Agent 1…"}
            disabled={isStreaming}
            rows={1}
            className="flex-1 resize-none border-none bg-transparent p-0 text-sm placeholder:text-muted-foreground/40 focus-visible:ring-0 focus-visible:ring-offset-0 min-h-[24px] max-h-[140px] leading-relaxed disabled:opacity-50"
          />
          <div className="flex items-center pb-0.5 shrink-0">
            {isStreaming ? (
              <button onClick={handleAbort}
                className="h-7 w-7 rounded-lg bg-rose-500/15 text-rose-400 flex items-center justify-center hover:bg-rose-500/25 transition-colors"
                title="Stop generation">
                <RotateCcw className="h-3.5 w-3.5" />
              </button>
            ) : (
              <button onClick={handleSend} disabled={!input.trim()}
                className={cn("h-7 w-7 rounded-lg flex items-center justify-center transition-all",
                  input.trim()
                    ? "bg-primary text-primary-foreground hover:bg-primary/90 shadow-md shadow-primary/20 active:scale-90"
                    : "bg-muted text-muted-foreground/40 cursor-not-allowed")}
                title="Send (Enter)">
                <ArrowUp className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>
        <p className="text-[10px] text-muted-foreground/25 font-mono mt-1.5 px-1">
          Enter to send · Shift+Enter for newline
        </p>
      </div>
    </div>
  );
}

