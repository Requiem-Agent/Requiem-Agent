import { useState, useRef, useEffect, useCallback } from "react";
import { AppLayout } from "@/components/layout";
import {
  useSessions, useSessionMutations, useMessageMutations, useMessages,
} from "@/hooks/use-sessions";
import { ROLE_MODEL_MAP, FREE_ZEN_MODELS } from "@/hooks/use-system";
import { FormattedMessage } from "@/components/message-formatter";
import { streamZenChat } from "@/lib/zen-client";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { SessionMode, SessionEffort } from "@workspace/api-client-react";
import { useToast } from "@/hooks/use-toast";
import {
  Terminal, Plus, X, Command, Code2, Settings2, Palette,
  Bug, Search, Map, Shield, Send, Loader2, Cpu, BrainCircuit,
  Wrench, CheckCircle2, ChevronDown, Zap, Layers, RotateCcw,
  Bot, Sparkles, ArrowUp,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { fetchRagContext, autoStoreMemory } from "@/hooks/use-memory";

// ── Mode icons ────────────────────────────────────────────────────────────────
const MODE_META: Record<SessionMode, { Icon: React.ElementType; label: string; color: string; desc: string }> = {
  orchestrator: { Icon: Settings2, label: "Orchestrator", color: "text-primary",     desc: "Coordinates all models" },
  coder:        { Icon: Code2,     label: "Coder",        color: "text-cyan-400",    desc: "Code generation & edits" },
  planner:      { Icon: Command,   label: "Planner",      color: "text-violet-400",  desc: "Architectural planning" },
  debugger:     { Icon: Bug,       label: "Debugger",     color: "text-rose-400",    desc: "Root-cause analysis" },
  designer:     { Icon: Palette,   label: "Designer",     color: "text-pink-400",    desc: "UI/UX & creative" },
  researcher:   { Icon: Search,    label: "Researcher",   color: "text-amber-400",   desc: "Deep research & analysis" },
  explorer:     { Icon: Map,       label: "Explorer",     color: "text-emerald-400", desc: "Codebase navigation" },
  security:     { Icon: Shield,    label: "Security",     color: "text-orange-400",  desc: "Vulnerability scanning" },
};

const EFFORT_META: Record<SessionEffort, { label: string; desc: string; color: string; bg: string }> = {
  lite:   { label: "Lite",   desc: "Fast & light",     color: "text-muted-foreground", bg: "bg-muted/60" },
  medium: { label: "Med",    desc: "Balanced",         color: "text-cyan-400",         bg: "bg-cyan-400/10" },
  high:   { label: "High",   desc: "Deep analysis",    color: "text-amber-400",        bg: "bg-amber-400/10" },
  max:    { label: "Max",    desc: "All models",       color: "text-rose-400",         bg: "bg-rose-400/10" },
};

// ── Thinking indicator ────────────────────────────────────────────────────────
function ThinkingIndicator({ mode }: { mode: SessionMode }) {
  const { Icon, color, label } = MODE_META[mode] || MODE_META.orchestrator;
  const stages = [
    "Analyzing context",
    "Retrieving memory",
    `${label} mode active`,
    "Generating response",
  ];
  const [stage, setStage] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setStage(s => (s + 1) % stages.length), 1400);
    return () => clearInterval(t);
  }, []);
  return (
    <div className="flex items-center gap-3 py-2 animate-fade-in">
      <div className={cn("h-7 w-7 rounded-lg flex items-center justify-center bg-card border border-border/60 shrink-0", color)}>
        <Icon className="h-3.5 w-3.5" />
      </div>
      <div className="flex flex-col gap-0.5">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground font-mono">{stages[stage]}</span>
          <span className="flex gap-0.5">
            {[0,1,2].map(i => (
              <span key={i} className="typing-dot h-1.5 w-1.5 rounded-full bg-primary/60 inline-block" />
            ))}
          </span>
        </div>
      </div>
    </div>
  );
}

// ── Message bubble ────────────────────────────────────────────────────────────
function MessageBubble({
  message, isNew,
}: {
  message: { id: string; role: string; content: string; modelUsed?: string | null; codeChanges?: string | null };
  isNew: boolean;
}) {
  const isUser = message.role === "user";
  const isThinking = message.role === "thinking";
  const isTool = message.role === "tool";

  if (isThinking) {
    return (
      <div className={cn("flex justify-start w-full", isNew && "animate-slide-up")}>
        <div className="bg-card/40 border border-border/40 rounded-xl px-3.5 py-2.5 max-w-[85%]">
          <div className="flex items-center gap-2 mb-1.5">
            <BrainCircuit className="h-3 w-3 text-violet-400" />
            <span className="text-[10px] font-mono text-violet-400 uppercase tracking-wider">thinking</span>
          </div>
          <p className="text-xs text-muted-foreground/70 italic leading-relaxed line-clamp-4">
            {message.content}
          </p>
        </div>
      </div>
    );
  }

  if (isTool) {
    return (
      <div className={cn("flex justify-start w-full", isNew && "animate-slide-up")}>
        <div className="bg-[#0a0c10] border border-cyan-500/20 rounded-lg px-3 py-2 flex items-center gap-3 text-xs max-w-[85%]">
          <Wrench className="h-3 w-3 text-cyan-500 shrink-0" />
          <span className="font-mono text-cyan-400 truncate">{message.content.substring(0, 80)}</span>
          <CheckCircle2 className="h-3 w-3 text-emerald-500 ml-auto shrink-0" />
        </div>
      </div>
    );
  }

  if (isUser) {
    return (
      <div className={cn("flex justify-end w-full", isNew && "animate-slide-up")}>
        <div className="msg-user border rounded-2xl rounded-br-sm px-4 py-2.5 max-w-[85%] text-sm leading-relaxed">
          {message.content}
        </div>
      </div>
    );
  }

  // Assistant
  return (
    <div className={cn("flex justify-start w-full", isNew && "animate-slide-up")}>
      <div className="w-full max-w-full">
        {/* Model badge */}
        {message.modelUsed && (
          <div className="flex items-center gap-1.5 mb-2">
            <div className="h-4 w-4 rounded bg-primary/10 flex items-center justify-center">
              <Bot className="h-2.5 w-2.5 text-primary" />
            </div>
            <span className="text-[10px] text-muted-foreground/50 font-mono">Requiem Agent 1</span>
          </div>
        )}

        <div className="msg-assistant border rounded-2xl rounded-tl-sm px-4 py-3 text-sm leading-relaxed">
          <FormattedMessage content={message.content} />
        </div>

        {message.codeChanges && (
          <div className="mt-2 bg-[#0a0c10] border border-border rounded-xl overflow-hidden">
            <div className="bg-[#0f1014] px-3 py-2 border-b border-border flex items-center text-xs font-mono gap-2">
              <Code2 className="h-3 w-3 text-primary" />
              <span className="text-muted-foreground">File Modifications</span>
            </div>
            <pre className="p-3 text-xs font-mono overflow-x-auto">
              <code>
                {message.codeChanges.split('\n').map((line: string, idx: number) => {
                  if (line.startsWith('+')) return <div key={idx} className="text-emerald-400 bg-emerald-400/10 px-1 rounded-sm">{line}</div>;
                  if (line.startsWith('-')) return <div key={idx} className="text-rose-400 bg-rose-400/10 px-1 rounded-sm">{line}</div>;
                  if (line.startsWith('@@')) return <div key={idx} className="text-cyan-400 my-1">{line}</div>;
                  return <div key={idx} className="text-muted-foreground px-1">{line}</div>;
                })}
              </code>
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

// ── Streaming message ─────────────────────────────────────────────────────────
function StreamingMessage({ content, thinking }: { content: string; thinking: string }) {
  return (
    <div className="flex justify-start w-full animate-fade-in">
      <div className="w-full max-w-full space-y-2">
        {thinking && (
          <div className="bg-card/40 border border-border/40 rounded-xl px-3.5 py-2.5">
            <div className="flex items-center gap-2 mb-1">
              <BrainCircuit className="h-3 w-3 text-violet-400" />
              <span className="text-[10px] font-mono text-violet-400 uppercase tracking-wider">thinking</span>
            </div>
            <p className="text-xs text-muted-foreground/60 italic leading-relaxed">{thinking}</p>
          </div>
        )}
        {content && (
          <div className="msg-assistant border rounded-2xl rounded-tl-sm px-4 py-3 text-sm leading-relaxed">
            <FormattedMessage content={content} />
            <span className="stream-cursor" />
          </div>
        )}
        {!content && !thinking && <ThinkingIndicator mode={SessionMode.orchestrator} />}
      </div>
    </div>
  );
}

// ── Empty state ───────────────────────────────────────────────────────────────
function EmptyChat({ mode, onPrompt }: { mode: SessionMode; onPrompt: (t: string) => void }) {
  const { Icon, label, color } = MODE_META[mode] || MODE_META.orchestrator;
  const suggestions = {
    coder:      ["Write a REST API in TypeScript", "Create a React component", "Refactor this function"],
    planner:    ["Plan a microservices architecture", "Break down this project", "Design a database schema"],
    debugger:   ["Debug this error", "Find memory leaks", "Trace this crash"],
    researcher: ["Explain this concept", "Compare these approaches", "Research best practices"],
    designer:   ["Design a landing page", "Create UI components", "Build a color system"],
    explorer:   ["Explore the codebase", "Find all API endpoints", "Map dependencies"],
    security:   ["Scan for vulnerabilities", "Review auth flow", "Check for SQL injection"],
    orchestrator: ["Build a full-stack app", "Coordinate complex task", "Multi-model analysis"],
  };
  const tips = suggestions[mode] || suggestions.orchestrator;

  return (
    <div className="flex flex-col items-center justify-center h-full px-6 text-center gap-6 animate-fade-in">
      {/* Icon */}
      <div className="relative">
        <div className={cn("h-16 w-16 rounded-2xl border flex items-center justify-center bg-card/60 shadow-lg", color.replace("text-", "border-").replace("400", "400/30"))}>
          <Icon className={cn("h-8 w-8", color)} />
        </div>
        <div className="absolute -bottom-1 -right-1 h-5 w-5 rounded-full bg-background border border-border flex items-center justify-center">
          <Sparkles className="h-2.5 w-2.5 text-primary" />
        </div>
      </div>

      {/* Title */}
      <div className="space-y-1.5">
        <h2 className="text-base font-semibold tracking-tight">
          {label} <span className="gradient-text">Ready</span>
        </h2>
        <p className="text-xs text-muted-foreground/60 max-w-56 leading-relaxed">
          Requiem Agent 1 is listening. Start a conversation or try a suggestion below.
        </p>
      </div>

      {/* Quick suggestions */}
      <div className="flex flex-col gap-2 w-full max-w-xs">
        {tips.map((tip, i) => (
          <button
            key={i}
            onClick={() => onPrompt(tip)}
            className="text-left text-xs px-3.5 py-2.5 rounded-xl border border-border/60 bg-card/30 hover:bg-card hover:border-primary/30 transition-all duration-150 text-muted-foreground hover:text-foreground"
          >
            {tip}
          </button>
        ))}
      </div>
    </div>
  );
}

// ── Session tab ───────────────────────────────────────────────────────────────
function SessionTab({
  session, isActive, onSelect, onDelete, onRename,
}: {
  session: any; isActive: boolean;
  onSelect: () => void; onDelete: (e: React.MouseEvent) => void;
  onRename: (id: string, name: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(session.name);
  const inputRef = useRef<HTMLInputElement>(null);
  const modeInfo = MODE_META[session.mode as SessionMode] || MODE_META.orchestrator;

  useEffect(() => { if (editing) inputRef.current?.focus(); }, [editing]);

  function commit() {
    setEditing(false);
    if (name.trim()) onRename(session.id, name.trim());
    else setName(session.name);
  }

  return (
    <button
      onClick={onSelect}
      className={cn(
        "flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-150 min-w-0 group relative shrink-0",
        isActive
          ? "bg-primary/12 text-primary border border-primary/25 shadow-sm"
          : "text-muted-foreground hover:text-foreground hover:bg-white/[0.04] border border-transparent"
      )}
    >
      <modeInfo.Icon className={cn("h-3 w-3 shrink-0", isActive ? modeInfo.color : "text-muted-foreground")} />

      {editing ? (
        <input
          ref={inputRef}
          value={name}
          onChange={e => setName(e.target.value)}
          onBlur={commit}
          onKeyDown={e => { if (e.key === "Enter") commit(); if (e.key === "Escape") { setEditing(false); setName(session.name); } }}
          onClick={e => e.stopPropagation()}
          className="bg-transparent border-none outline-none text-xs w-24 text-foreground"
        />
      ) : (
        <span
          className="truncate max-w-[80px]"
          onDoubleClick={e => { e.stopPropagation(); setEditing(true); }}
        >
          {session.name}
        </span>
      )}

      <button
        onClick={onDelete}
        className={cn(
          "ml-0.5 p-0.5 rounded opacity-0 group-hover:opacity-100 transition-opacity hover:text-destructive shrink-0",
          isActive ? "opacity-60" : ""
        )}
      >
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

  // Set initial session
  useEffect(() => {
    if (sessions.length > 0 && !activeSessionId && !sessionsLoading) {
      setActiveSessionId(sessions[0].id);
    } else if (sessions.length === 0 && !sessionsLoading) {
      setActiveSessionId(null);
    }
  }, [sessions, sessionsLoading]);

  useEffect(() => {
    if (activeSessionId && sessions.length > 0) {
      const exists = sessions.find(s => s.id === activeSessionId);
      if (!exists) setActiveSessionId(sessions[0].id);
    }
  }, [sessions, activeSessionId]);

  const activeSession = sessions.find(s => s.id === activeSessionId);

  async function handleCreateSession() {
    if (sessions.length >= 3) {
      toast({ title: "Limit reached", description: "Max 3 sessions. Delete one first.", variant: "destructive" });
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
    try { await update({ id, data: { name } }); } catch {}
  }

  async function handleChangeMode(mode: SessionMode) {
    if (!activeSession) return;
    try { await update({ id: activeSession.id, data: { mode } }); setShowModePanel(false); } catch {}
  }

  async function handleChangeEffort(effort: SessionEffort) {
    if (!activeSession) return;
    try { await update({ id: activeSession.id, data: { effort } }); setShowEffortPanel(false); } catch {}
  }

  // Close panels on outside click
  useEffect(() => {
    if (!showModePanel && !showEffortPanel) return;
    const handler = () => { setShowModePanel(false); setShowEffortPanel(false); };
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, [showModePanel, showEffortPanel]);

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-hidden">
        {/* ── Session bar ── */}
        <div className="shrink-0 flex items-center gap-2 px-3 pt-2 pb-1.5 border-b border-border/50 overflow-x-auto scrollbar-none">
          {sessionsLoading ? (
            <div className="h-7 w-24 rounded-lg animate-shimmer" />
          ) : (
            <>
              {sessions.map(s => (
                <SessionTab
                  key={s.id}
                  session={s}
                  isActive={s.id === activeSessionId}
                  onSelect={() => setActiveSessionId(s.id)}
                  onDelete={(e) => handleDeleteSession(s.id, e)}
                  onRename={handleRenameSession}
                />
              ))}
              {sessions.length < 3 && (
                <button
                  onClick={handleCreateSession}
                  disabled={isCreating}
                  className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs text-muted-foreground hover:text-foreground border border-dashed border-border/50 hover:border-border transition-all"
                >
                  {isCreating
                    ? <Loader2 className="h-3 w-3 animate-spin" />
                    : <Plus className="h-3 w-3" />
                  }
                  <span>New</span>
                </button>
              )}
            </>
          )}
        </div>

        {/* ── Mode/Effort toolbar (only when session active) ── */}
        {activeSession && (
          <div className="shrink-0 flex items-center gap-2 px-3 py-1.5 border-b border-border/30">
            {/* Mode selector */}
            <div className="relative" onClick={e => e.stopPropagation()}>
              <button
                onClick={() => { setShowModePanel(p => !p); setShowEffortPanel(false); }}
                className={cn(
                  "flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs border transition-all",
                  showModePanel
                    ? "bg-primary/10 border-primary/30 text-primary"
                    : "bg-card/40 border-border/50 text-muted-foreground hover:text-foreground hover:border-border"
                )}
              >
                {(() => { const m = MODE_META[activeSession.mode as SessionMode] || MODE_META.orchestrator; return <m.Icon className={cn("h-3 w-3", m.color)} />; })()}
                <span className="capitalize">{MODE_META[activeSession.mode as SessionMode]?.label || activeSession.mode}</span>
                <ChevronDown className="h-2.5 w-2.5" />
              </button>

              {showModePanel && (
                <div className="absolute top-full left-0 mt-1.5 bg-card border border-border rounded-xl shadow-xl z-50 p-1.5 grid grid-cols-2 gap-0.5 w-56 animate-scale-in">
                  {Object.entries(MODE_META).map(([key, { Icon, label, color, desc }]) => (
                    <button
                      key={key}
                      onClick={() => handleChangeMode(key as SessionMode)}
                      className={cn(
                        "flex items-center gap-2 px-2.5 py-2 rounded-lg text-xs transition-all text-left",
                        activeSession.mode === key
                          ? "bg-primary/10 text-primary"
                          : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground"
                      )}
                    >
                      <Icon className={cn("h-3 w-3 shrink-0", color)} />
                      <span className="font-medium">{label}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* Effort selector */}
            <div className="relative" onClick={e => e.stopPropagation()}>
              <button
                onClick={() => { setShowEffortPanel(p => !p); setShowModePanel(false); }}
                className={cn(
                  "flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs border transition-all",
                  showEffortPanel
                    ? "bg-primary/10 border-primary/30 text-primary"
                    : "bg-card/40 border-border/50 text-muted-foreground hover:text-foreground hover:border-border"
                )}
              >
                <Zap className={cn("h-3 w-3", EFFORT_META[activeSession.effort as SessionEffort]?.color || "text-muted-foreground")} />
                <span>{EFFORT_META[activeSession.effort as SessionEffort]?.label || activeSession.effort}</span>
                <ChevronDown className="h-2.5 w-2.5" />
              </button>

              {showEffortPanel && (
                <div className="absolute top-full left-0 mt-1.5 bg-card border border-border rounded-xl shadow-xl z-50 p-1.5 w-44 animate-scale-in">
                  {Object.entries(EFFORT_META).map(([key, { label, desc, color }]) => (
                    <button
                      key={key}
                      onClick={() => handleChangeEffort(key as SessionEffort)}
                      className={cn(
                        "flex items-center justify-between w-full px-2.5 py-2 rounded-lg text-xs transition-all",
                        activeSession.effort === key
                          ? "bg-primary/10 text-primary"
                          : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground"
                      )}
                    >
                      <span className={cn("font-medium", color)}>{label}</span>
                      <span className="text-muted-foreground/50">{desc}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div className="ml-auto flex items-center gap-1.5 text-[10px] text-muted-foreground/40 font-mono">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-400/60" />
              online
            </div>
          </div>
        )}

        {/* ── Chat area ── */}
        {activeSessionId ? (
          <ChatPanel
            sessionId={activeSessionId}
            mode={(activeSession?.mode as SessionMode) || SessionMode.orchestrator}
            effort={(activeSession?.effort as SessionEffort) || SessionEffort.medium}
          />
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center gap-5 px-6 text-center animate-fade-in">
            <div className="h-14 w-14 rounded-2xl bg-primary/8 border border-primary/15 flex items-center justify-center animate-float">
              <Terminal className="h-7 w-7 text-primary" />
            </div>
            <div className="space-y-1.5">
              <h1 className="text-lg font-semibold tracking-tight gradient-text">Requiem Agent 1</h1>
              <p className="text-xs text-muted-foreground/60 max-w-52">
                Create a session to start chatting with the AI agent.
              </p>
            </div>
            <button
              onClick={handleCreateSession}
              disabled={isCreating}
              className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-lg shadow-primary/20"
            >
              {isCreating ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
              New Session
            </button>
          </div>
        )}
      </div>
    </AppLayout>
  );
}

// ── Chat Panel (isolated) ─────────────────────────────────────────────────────
function ChatPanel({
  sessionId, mode, effort,
}: {
  sessionId: string; mode: SessionMode; effort: SessionEffort;
}) {
  const { data: messages = [], isLoading } = useMessages(sessionId);
  const { addMessage } = useMessageMutations();
  const { toast } = useToast();

  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState("");
  const [streamingThinking, setStreamingThinking] = useState("");
  const [newMessageIds, setNewMessageIds] = useState<Set<string>>(new Set());

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const abortRef = useRef<AbortController | null>(null);

  // Auto-scroll
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingContent]);

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
    setStreamingContent("");
    setStreamingThinking("");

    try {
      // Save user message
      const userMsg = await addMessage(sessionId, {
        role: "user", content: text, mode, effort,
      });
      setNewMessageIds(prev => new Set(prev).add(userMsg.id));

      // Build context
      const ragCtx = await fetchRagContext(text, sessionId, 1200);
      const systemPrompt = [
        "You are Requiem Agent 1 — a powerful AI coding and research assistant.",
        ragCtx?.systemContext ? `\n\nMemory context:\n${ragCtx.systemContext}` : "",
      ].join("").trim();

      const historyMsgs = messages.slice(-12).filter(m => m.role === "user" || m.role === "assistant");
      const contextMessages = [
        { role: "system", content: systemPrompt },
        ...historyMsgs.map(m => ({ role: m.role, content: m.content })),
        { role: "user", content: text },
      ];

      // Stream
      abortRef.current = new AbortController();
      let full = "";
      let thinking = "";
      let tokenCount = 0;

      for await (const chunk of streamZenChat(modelId, contextMessages, abortRef.current.signal)) {
        full += chunk;
        tokenCount++;
        // First 300 tokens of high/max effort → thinking
        if ((effort === "high" || effort === "max") && tokenCount <= 40) {
          thinking += chunk;
          setStreamingThinking(thinking);
        } else {
          setStreamingContent(full.slice(thinking.length));
        }
      }

      // Save assistant message
      const assistantMsg = await addMessage(sessionId, {
        role: "assistant",
        content: full,
        modelUsed: modelId,
        mode,
        effort,
      });
      setNewMessageIds(prev => new Set(prev).add(assistantMsg.id));

      // Auto-store memory (fire & forget)
      autoStoreMemory(text, full, sessionId);

    } catch (err: any) {
      if (err.name !== "AbortError") {
        toast({ title: "Error", description: err.message || "Failed to send message.", variant: "destructive" });
      }
    } finally {
      setIsStreaming(false);
      setStreamingContent("");
      setStreamingThinking("");
    }
  }

  function handleAbort() {
    abortRef.current?.abort();
    setIsStreaming(false);
    setStreamingContent("");
    setStreamingThinking("");
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  function handleSuggestion(text: string) {
    setInput(text);
    textareaRef.current?.focus();
  }

  return (
    <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
      {/* Messages scroll area */}
      <div className="flex-1 overflow-y-auto min-h-0 px-3 py-3 space-y-3">
        {isLoading ? (
          <div className="flex items-center justify-center py-10">
            <Loader2 className="h-5 w-5 animate-spin text-primary/60" />
          </div>
        ) : messages.length === 0 && !isStreaming ? (
          <EmptyChat mode={mode} onPrompt={handleSuggestion} />
        ) : (
          <>
            {messages.map(m => (
              <MessageBubble
                key={m.id}
                message={m as any}
                isNew={newMessageIds.has(m.id)}
              />
            ))}
            {isStreaming && (
              <StreamingMessage content={streamingContent} thinking={streamingThinking} />
            )}
          </>
        )}
        <div ref={messagesEndRef} className="h-1" />
      </div>

      {/* Prompt input — never overlaps messages */}
      <div className="shrink-0 px-3 pb-3 pt-2 border-t border-border/40">
        <div
          className={cn(
            "flex items-end gap-2 rounded-2xl border bg-card/60 px-3 py-2 transition-all duration-200",
            isStreaming ? "border-primary/30" : "border-border/60 focus-within:border-primary/40"
          )}
        >
          <Textarea
            ref={textareaRef}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={isStreaming ? "Agent is responding..." : "Message Requiem Agent 1…"}
            disabled={isStreaming}
            rows={1}
            className="flex-1 resize-none border-none bg-transparent p-0 text-sm placeholder:text-muted-foreground/40 focus-visible:ring-0 focus-visible:ring-offset-0 min-h-[24px] max-h-[140px] leading-relaxed disabled:opacity-50"
          />
          <div className="flex items-center gap-1.5 pb-0.5 shrink-0">
            {isStreaming ? (
              <button
                onClick={handleAbort}
                className="h-7 w-7 rounded-lg bg-rose-500/15 text-rose-400 flex items-center justify-center hover:bg-rose-500/25 transition-colors"
                title="Stop"
              >
                <RotateCcw className="h-3.5 w-3.5" />
              </button>
            ) : (
              <button
                onClick={handleSend}
                disabled={!input.trim()}
                className={cn(
                  "h-7 w-7 rounded-lg flex items-center justify-center transition-all duration-150",
                  input.trim()
                    ? "bg-primary text-primary-foreground hover:bg-primary/90 shadow-md shadow-primary/20 active:scale-90"
                    : "bg-muted text-muted-foreground/40 cursor-not-allowed"
                )}
                title="Send (Enter)"
              >
                <ArrowUp className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>

        <div className="flex items-center justify-between mt-1.5 px-1">
          <span className="text-[10px] text-muted-foreground/30 font-mono">Enter to send · Shift+Enter for newline</span>
          <span className="text-[10px] text-muted-foreground/30 font-mono">
            {FREE_ZEN_MODELS.find(m => m.id === modelId)?.name || "Requiem Agent 1"}
          </span>
        </div>
      </div>
    </div>
  );
}
