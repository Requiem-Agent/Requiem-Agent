import { useState, useRef, useEffect } from "react";
import { AppLayout } from "@/components/layout";
import { useSessions, useSessionMutations, useSession, useMessageMutations, useMessages } from "@/hooks/use-sessions";
import { ROLE_MODEL_MAP, FREE_ZEN_MODELS } from "@/hooks/use-system";
import { FormattedMessage, TypewriterText } from "@/components/message-formatter";
import { streamZenChat } from "@/lib/zen-client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { SessionMode, SessionEffort } from "@workspace/api-client-react";
import { useToast } from "@/hooks/use-toast";
import { 
  Terminal, Plus, X, Command, Code2, 
  Settings2, Palette, Bug, Search, 
  Map, Shield, Send, Loader2, Cpu, BrainCircuit, Wrench, CheckCircle2
} from "lucide-react";
import { cn } from "@/lib/utils";

const MODE_ICONS: Record<SessionMode, React.ElementType> = {
  planner: Command,
  coder: Code2,
  orchestrator: Settings2,
  designer: Palette,
  debugger: Bug,
  researcher: Search,
  explorer: Map,
  security: Shield,
};

const EFFORT_COLORS: Record<SessionEffort, string> = {
  lite: "text-muted-foreground bg-muted border-border hover:bg-muted/80",
  medium: "text-cyan-400 bg-cyan-400/10 border-cyan-400/30 hover:bg-cyan-400/20",
  high: "text-amber-400 bg-amber-400/10 border-amber-400/30 hover:bg-amber-400/20",
  max: "text-destructive bg-destructive/10 border-destructive/30 hover:bg-destructive/20 shadow-[0_0_15px_rgba(239,68,68,0.2)]",
};

export default function WorkspacePage() {
  const { data: sessions = [], isLoading: sessionsLoading } = useSessions();
  const { create, update, remove, isCreating } = useSessionMutations();
  const { toast } = useToast();

  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  
  // Set initial active session
  useEffect(() => {
    if (sessions.length > 0 && !activeSessionId && !sessionsLoading) {
      setActiveSessionId(sessions[0].id);
    } else if (sessions.length === 0 && !sessionsLoading) {
      setActiveSessionId(null);
    }
  }, [sessions, activeSessionId, sessionsLoading]);

  // Handle session selection to ensure it exists
  useEffect(() => {
    if (activeSessionId && sessions.length > 0) {
      const exists = sessions.find(s => s.id === activeSessionId);
      if (!exists) {
        setActiveSessionId(sessions[0].id);
      }
    }
  }, [sessions, activeSessionId]);

  const activeSession = sessions.find(s => s.id === activeSessionId);

  async function handleCreateSession() {
    if (sessions.length >= 3) {
      toast({ title: "Limit reached", description: "You can only have 3 active sessions. Delete one first.", variant: "destructive" });
      return;
    }
    try {
      const newSession = await create({
        name: `Session ${sessions.length + 1}`,
        mode: SessionMode.coder,
        effort: SessionEffort.medium,
      });
      setActiveSessionId(newSession.id);
    } catch (e: any) {
      toast({ title: "Error", description: "Failed to create session.", variant: "destructive" });
    }
  }

  async function handleDeleteSession(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    try {
      await remove(id);
      if (activeSessionId === id) {
        setActiveSessionId(null);
      }
    } catch (e: any) {
      toast({ title: "Error", description: "Failed to delete session.", variant: "destructive" });
    }
  }

  async function handleRenameSession(id: string) {
    if (!editingName.trim()) {
      setEditingSessionId(null);
      return;
    }
    try {
      await update(id, { name: editingName.trim() });
      setEditingSessionId(null);
    } catch (e: any) {
      toast({ title: "Error", description: "Failed to rename session.", variant: "destructive" });
    }
  }

  async function handleUpdateMode(mode: SessionMode) {
    if (!activeSessionId || !activeSession) return;
    try {
      await update(activeSessionId, { mode });
    } catch (e: any) {
      // Ignore error for optimistic-like feel
    }
  }

  async function handleUpdateEffort(effort: SessionEffort) {
    if (!activeSessionId || !activeSession) return;
    try {
      await update(activeSessionId, { effort });
    } catch (e: any) {
      // Ignore
    }
  }

  // Active Model Derivation
  const activeModelId = activeSession?.activeModel || ROLE_MODEL_MAP[activeSession?.mode ?? "orchestrator"] || "deepseek-v4-flash-free";
  const activeModel = FREE_ZEN_MODELS.find(m => m.id === activeModelId);

  return (
    <AppLayout>
      <div className="flex flex-col h-full bg-[#0d0d0f] relative overflow-hidden">
        
        {/* Top Session Tabs */}
        <div className="h-12 border-b border-border/50 bg-[#141416] flex items-center px-2 shrink-0 overflow-x-auto hide-scrollbar z-10 relative">
          <div className="flex items-center gap-1.5 h-full">
            {sessionsLoading ? (
              <div className="flex items-center px-4 py-1 text-sm text-muted-foreground"><Loader2 className="h-3 w-3 animate-spin mr-2" /> Loading sessions...</div>
            ) : (
              <>
                {sessions.map((session) => {
                  const isActive = activeSessionId === session.id;
                  const Icon = MODE_ICONS[session.mode as SessionMode] || Terminal;
                  const isEditing = editingSessionId === session.id;
                  
                  return (
                    <div 
                      key={session.id}
                      onClick={() => !isEditing && setActiveSessionId(session.id)}
                      className={cn(
                        "group flex items-center h-9 px-3 min-w-[140px] max-w-[200px] rounded-md border border-transparent transition-all cursor-pointer select-none",
                        isActive 
                          ? "bg-card border-border/80 text-foreground shadow-sm" 
                          : "text-muted-foreground hover:bg-white/[0.03] hover:text-foreground"
                      )}
                    >
                      <Icon className={cn("h-3.5 w-3.5 shrink-0 mr-2", isActive ? "text-primary" : "")} />
                      
                      {isEditing ? (
                        <input
                          autoFocus
                          className="flex-1 bg-transparent border-none outline-none text-sm font-medium p-0 h-full min-w-0"
                          value={editingName}
                          onChange={(e) => setEditingName(e.target.value)}
                          onBlur={() => handleRenameSession(session.id)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') handleRenameSession(session.id);
                            if (e.key === 'Escape') setEditingSessionId(null);
                          }}
                        />
                      ) : (
                        <span 
                          className="flex-1 truncate text-sm font-medium"
                          onDoubleClick={() => {
                            setEditingName(session.name);
                            setEditingSessionId(session.id);
                          }}
                        >
                          {session.name}
                        </span>
                      )}
                      
                      <button 
                        className={cn(
                          "ml-1 shrink-0 p-1 rounded-sm opacity-0 group-hover:opacity-100 transition-opacity focus:opacity-100 outline-none",
                          isActive ? "hover:bg-muted" : "hover:bg-white/10"
                        )}
                        onClick={(e) => handleDeleteSession(session.id, e)}
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </div>
                  );
                })}
                
                {sessions.length < 3 && (
                  <button 
                    className="flex items-center justify-center h-9 w-9 rounded-md border border-dashed border-border/50 text-muted-foreground hover:text-foreground hover:border-border hover:bg-white/[0.02] transition-colors ml-1"
                    onClick={handleCreateSession}
                    disabled={isCreating}
                  >
                    {isCreating ? <Loader2 className="h-3 w-3 animate-spin" /> : <Plus className="h-4 w-4" />}
                  </button>
                )}
              </>
            )}
          </div>
        </div>

        {/* Main Content Area */}
        {activeSessionId && activeSession ? (
          <div className="flex-1 flex overflow-hidden">
            {/* Left Panel: Configuration */}
            <div className="w-64 shrink-0 border-r border-border/50 bg-[#0a0a0c] flex flex-col hidden md:flex z-10 relative">
              
              <div className="p-4 border-b border-border/50">
                <h3 className="text-[10px] uppercase font-mono tracking-widest text-muted-foreground mb-3 flex items-center">
                  <Cpu className="h-3 w-3 mr-1.5" /> Agent Mode
                </h3>
                <div className="flex flex-col gap-1.5">
                  {Object.keys(SessionMode).map((modeKey) => {
                    const mode = modeKey as SessionMode;
                    const Icon = MODE_ICONS[mode];
                    const isActive = activeSession.mode === mode;
                    return (
                      <button
                        key={mode}
                        onClick={() => handleUpdateMode(mode)}
                        className={cn(
                          "flex items-center px-2.5 py-1.5 rounded-md text-sm transition-colors text-left",
                          isActive 
                            ? "bg-primary/15 text-primary font-medium border border-primary/20" 
                            : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground border border-transparent"
                        )}
                      >
                        <Icon className="h-4 w-4 mr-2.5 shrink-0" />
                        <span className="capitalize">{mode}</span>
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="p-4 border-b border-border/50">
                <h3 className="text-[10px] uppercase font-mono tracking-widest text-muted-foreground mb-3 flex items-center">
                  <ActivityIcon className="h-3 w-3 mr-1.5" /> Compute Effort
                </h3>
                <div className="grid grid-cols-2 gap-1.5">
                  {Object.keys(SessionEffort).map((effortKey) => {
                    const effort = effortKey as SessionEffort;
                    const isActive = activeSession.effort === effort;
                    return (
                      <button
                        key={effort}
                        onClick={() => handleUpdateEffort(effort)}
                        className={cn(
                          "px-2 py-1.5 rounded border text-xs font-mono font-medium transition-all text-center uppercase tracking-wide",
                          isActive ? EFFORT_COLORS[effort] : "bg-transparent border-border/50 text-muted-foreground hover:bg-white/[0.02]"
                        )}
                      >
                        {effort}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="p-4 mt-auto">
                <h3 className="text-[10px] uppercase font-mono tracking-widest text-muted-foreground mb-2 flex items-center">
                  <BrainCircuit className="h-3 w-3 mr-1.5" /> Active Model
                </h3>
                <div className="p-2 rounded-md bg-[#141416] border border-border/50 flex flex-col">
                  <span className="text-sm font-medium text-foreground truncate">{activeModel?.name || "Loading..."}</span>
                  <span className="text-[10px] font-mono text-muted-foreground mt-0.5 truncate">{activeModelId}</span>
                </div>
              </div>
            </div>

            {/* Center Panel: Chat */}
            <div className="flex-1 flex flex-col min-w-0 bg-[#0d0d0f] relative">
              <ChatInterface 
                sessionId={activeSessionId} 
                sessionMode={activeSession.mode} 
                sessionEffort={activeSession.effort}
                activeModelId={activeModelId}
              />
            </div>
            
          </div>
        ) : (
          <div className="flex-1 flex items-center justify-center text-muted-foreground flex-col">
            {!sessionsLoading && sessions.length === 0 ? (
              <>
                <Terminal className="h-12 w-12 mb-4 opacity-20" />
                <p>No active sessions. Create one to begin.</p>
                <Button onClick={handleCreateSession} className="mt-4" variant="outline">
                  <Plus className="h-4 w-4 mr-2" /> New Workspace
                </Button>
              </>
            ) : (
              <Loader2 className="h-8 w-8 animate-spin opacity-50" />
            )}
          </div>
        )}
      </div>
    </AppLayout>
  );
}

function ChatInterface({ sessionId, sessionMode, sessionEffort, activeModelId }: { sessionId: string, sessionMode: string, sessionEffort: string, activeModelId: string }) {
  const { data: messages = [], isLoading } = useMessages(sessionId);
  const { add } = useMessageMutations(sessionId);
  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState("");
  const [streamingThinking, setStreamingThinking] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const { toast } = useToast();

  // Scroll to bottom on new messages
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, streamingContent, streamingThinking]);

  async function handleSubmit(e?: React.FormEvent) {
    if (e) e.preventDefault();
    if (!input.trim() || isStreaming) return;

    const userMessageContent = input.trim();
    setInput("");
    
    // Save user message to DB
    try {
      await add({
        role: "user",
        content: userMessageContent,
        mode: sessionMode,
        effort: sessionEffort,
        modelUsed: activeModelId
      });
    } catch (e: any) {
      toast({ title: "Error", description: "Failed to save message.", variant: "destructive" });
      return;
    }

    // Build context for API
    const contextMessages = messages.map(m => ({
      role: m.role === "thinking" || m.role === "tool" ? "assistant" : m.role,
      content: m.content
    })).concat([{ role: "user", content: userMessageContent }]);

    setIsStreaming(true);
    setStreamingContent("");
    setStreamingThinking("");

    let fullContent = "";
    let isThinkingMode = sessionEffort === 'high' || sessionEffort === 'max';
    let thinkingBuffer = "";

    try {
      const abortController = new AbortController();
      const stream = streamZenChat(activeModelId, contextMessages as any, abortController.signal);
      
      for await (const chunk of stream) {
        if (isThinkingMode) {
          // Simplistic extraction of thinking vs actual response for visual flair
          // Real models might use specific tags, but we'll simulate it based on length
          if (thinkingBuffer.length < 500 && !chunk.includes('```')) {
             thinkingBuffer += chunk;
             setStreamingThinking(prev => prev + chunk);
          } else {
             isThinkingMode = false;
             fullContent += chunk;
             setStreamingContent(prev => prev + chunk);
          }
        } else {
          fullContent += chunk;
          setStreamingContent(prev => prev + chunk);
        }
      }

      // Save assistant message to DB
      if (fullContent) {
        await add({
          role: "assistant",
          content: fullContent,
          mode: sessionMode,
          effort: sessionEffort,
          modelUsed: activeModelId
        });
      }
      
      if (thinkingBuffer) {
        await add({
          role: "thinking",
          content: thinkingBuffer,
          mode: sessionMode,
          effort: sessionEffort,
          modelUsed: activeModelId
        });
      }

    } catch (error: any) {
      toast({ title: "API Error", description: error.message, variant: "destructive" });
    } finally {
      setIsStreaming(false);
      setStreamingContent("");
      setStreamingThinking("");
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <>
      <div 
        ref={scrollRef}
        className="flex-1 overflow-y-auto p-4 md:p-6 space-y-6 pb-32"
      >
        {isLoading && messages.length === 0 ? (
          <div className="flex justify-center py-8"><Loader2 className="h-6 w-6 animate-spin text-muted-foreground" /></div>
        ) : messages.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-muted-foreground opacity-50">
            <Terminal className="h-16 w-16 mb-4" />
            <p className="font-mono text-sm tracking-widest uppercase">System Ready</p>
          </div>
        ) : (
          messages.map((msg, i) => (
            <MessageBubble key={msg.id || i} message={msg} />
          ))
        )}

        {/* Streaming States */}
        {streamingThinking && (
          <div className="flex justify-start w-full">
            <div className="w-full max-w-3xl bg-[#141416] border border-border/50 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-2 text-muted-foreground text-xs font-mono uppercase tracking-widest">
                <BrainCircuit className="h-3 w-3 animate-pulse" /> Analyzing context
              </div>
              <div className="font-mono text-xs text-muted-foreground/80 leading-relaxed whitespace-pre-wrap opacity-70">
                {streamingThinking}
              </div>
            </div>
          </div>
        )}

        {streamingContent && (
          <div className="flex justify-start w-full">
            <div className="w-full max-w-3xl">
              <FormattedMessage content={streamingContent} />
            </div>
          </div>
        )}
      </div>

      {/* Input Area */}
      <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-[#0d0d0f] via-[#0d0d0f] to-transparent pt-10 pb-4 px-4 md:px-8">
        <div className="max-w-4xl mx-auto relative">
          <form 
            onSubmit={handleSubmit}
            className="relative bg-[#141416] border border-border rounded-xl shadow-lg focus-within:ring-1 focus-within:ring-primary focus-within:border-primary transition-all"
          >
            <div className="absolute top-3 left-4 flex gap-2 z-10 pointer-events-none">
              <Badge variant="outline" className="h-5 text-[9px] uppercase font-mono tracking-widest bg-[#0a0a0c]">
                {sessionMode}
              </Badge>
              <Badge variant="outline" className={cn("h-5 text-[9px] uppercase font-mono tracking-widest", sessionEffort === 'max' ? "text-destructive border-destructive/50" : sessionEffort === 'high' ? "text-amber-400 border-amber-400/50" : "text-muted-foreground")}>
                {sessionEffort}
              </Badge>
            </div>
            
            <Textarea 
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Enter directive... (Ctrl+Enter to submit)"
              className="min-h-[100px] max-h-[400px] w-full resize-none border-0 bg-transparent focus-visible:ring-0 pt-10 px-4 pb-12 shadow-none font-mono text-sm"
              disabled={isStreaming}
            />
            
            <div className="absolute bottom-3 right-3 flex items-center gap-2">
              <span className="text-[10px] text-muted-foreground font-mono hidden sm:inline-block">CTRL+ENTER</span>
              <Button 
                type="submit" 
                size="sm" 
                disabled={!input.trim() || isStreaming}
                className="h-8 w-8 p-0 bg-primary hover:bg-primary/90 text-primary-foreground transition-all disabled:opacity-30"
              >
                {isStreaming ? <Loader2 className="h-4 w-4 animate-spin" /> : <Send className="h-4 w-4" />}
              </Button>
            </div>
          </form>
        </div>
      </div>
    </>
  );
}

function MessageBubble({ message }: { message: any }) {
  if (message.role === "user") {
    return (
      <div className="flex justify-end w-full">
        <div className="max-w-[85%] bg-primary/10 border border-primary/20 text-foreground rounded-2xl rounded-tr-sm px-4 py-3 text-sm">
          {message.content}
        </div>
      </div>
    );
  }

  if (message.role === "thinking") {
    return (
      <div className="flex justify-start w-full">
        <details className="w-full max-w-3xl bg-[#141416]/50 border border-border/30 rounded-lg group cursor-pointer">
          <summary className="p-3 text-xs font-mono uppercase tracking-widest text-muted-foreground flex items-center outline-none list-none">
            <BrainCircuit className="h-3 w-3 mr-2 group-open:text-primary transition-colors" /> 
            Reasoning Engine
            <span className="ml-auto opacity-50 group-open:rotate-180 transition-transform">▼</span>
          </summary>
          <div className="px-4 pb-4 pt-1 font-mono text-xs text-muted-foreground/70 leading-relaxed whitespace-pre-wrap border-t border-border/30">
            {message.content}
          </div>
        </details>
      </div>
    );
  }

  if (message.role === "tool") {
    return (
      <div className="flex justify-start w-full">
        <div className="bg-[#0a0a0c] border border-cyan-500/20 rounded-md px-3 py-2 flex items-center gap-3 text-xs">
          <Wrench className="h-3 w-3 text-cyan-500" />
          <span className="font-mono text-cyan-400">{message.content.substring(0, 50)}...</span>
          <CheckCircle2 className="h-3 w-3 text-emerald-500 ml-2" />
        </div>
      </div>
    );
  }

  // Assistant
  return (
    <div className="flex justify-start w-full">
      <div className="w-full max-w-3xl">
        <FormattedMessage content={message.content} />
        
        {message.codeChanges && (
          <div className="mt-4 bg-[#0a0a0c] border border-border rounded-lg overflow-hidden">
            <div className="bg-[#141416] px-3 py-2 border-b border-border flex items-center text-xs font-mono">
              <Code2 className="h-3 w-3 mr-2 text-primary" />
              File Modifications
            </div>
            <pre className="p-3 text-xs font-mono overflow-x-auto">
              <code>
                {message.codeChanges.split('\n').map((line: string, idx: number) => {
                  if (line.startsWith('+')) return <div key={idx} className="text-emerald-400 bg-emerald-400/10 px-1">{line}</div>;
                  if (line.startsWith('-')) return <div key={idx} className="text-destructive bg-destructive/10 px-1">{line}</div>;
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

function ActivityIcon(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}>
      <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
    </svg>
  );
}
