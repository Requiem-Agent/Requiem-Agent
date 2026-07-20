import { useState, useRef } from "react";
import { AppLayout } from "@/components/layout";
import { useExecCode, type SandboxLanguage } from "@/hooks/use-sandbox";
import { useToast } from "@/hooks/use-toast";
import {
  Terminal, Play, Loader2, Code2, CheckCircle2,
  AlertCircle, Clock, ChevronDown, Trash2, Copy, X,
} from "lucide-react";
import { cn } from "@/lib/utils";

const LANGUAGES: { id: SandboxLanguage; label: string; color: string; placeholder: string }[] = [
  {
    id: "python",
    label: "Python",
    color: "text-yellow-400",
    placeholder: `# Python sandbox
print("Hello from Requiem Agent!")

import sys
print(f"Python {sys.version}")

# Simple calculation
result = sum(range(1, 101))
print(f"Sum 1-100: {result}")`,
  },
  {
    id: "javascript",
    label: "JavaScript",
    color: "text-cyan-400",
    placeholder: `// JavaScript / Node.js sandbox
console.log("Hello from Requiem Agent!");

const data = [1, 2, 3, 4, 5];
const doubled = data.map(x => x * 2);
console.log("Doubled:", doubled);

// Async example
async function fetchData() {
  const res = await fetch("https://httpbin.org/json").catch(() => null);
  return res ? await res.json() : { error: "fetch failed" };
}
fetchData().then(console.log);`,
  },
  {
    id: "typescript",
    label: "TypeScript",
    color: "text-blue-400",
    placeholder: `// TypeScript sandbox
interface Agent {
  name: string;
  version: number;
  capabilities: string[];
}

const agent: Agent = {
  name: "Requiem Agent",
  version: 1,
  capabilities: ["code", "plan", "debug", "research"],
};

console.log(\`Agent: \${agent.name} v\${agent.version}\`);
agent.capabilities.forEach(cap => console.log(\` - \${cap}\`));`,
  },
  {
    id: "bash",
    label: "Bash",
    color: "text-emerald-400",
    placeholder: `#!/bin/bash
echo "Hello from Requiem Agent!"
echo ""
echo "System info:"
uname -a
echo ""
echo "Current directory:"
pwd
echo ""
echo "Files:"
ls -la /tmp 2>/dev/null | head -10`,
  },
];

interface ExecutionResult {
  success: boolean;
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
  language: string;
  compilation_error: string | null;
  timed_out: boolean;
}

export default function SandboxPage() {
  const [language, setLanguage] = useState<SandboxLanguage>("python");
  const [code, setCode] = useState("");
  const [result, setResult] = useState<ExecutionResult | null>(null);
  const [showLangPicker, setShowLangPicker] = useState(false);
  const exec = useExecCode();
  const { toast } = useToast();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const activeLang = LANGUAGES.find(l => l.id === language)!;

  async function handleRun() {
    const src = code.trim();
    if (!src) {
      toast({ title: "No code to run", variant: "destructive" });
      return;
    }
    try {
      const res = await exec.mutateAsync({ code: src, language, timeout_secs: 30 });
      setResult(res);
    } catch (err: any) {
      toast({ title: "Execution failed", description: err.message, variant: "destructive" });
    }
  }

  function handleCopy(text: string) {
    navigator.clipboard.writeText(text).then(() => {
      toast({ title: "Copied!" });
    });
  }

  function handleClear() {
    setCode("");
    setResult(null);
    textareaRef.current?.focus();
  }

  const hasOutput = result !== null;

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 pt-4 pb-6 space-y-4 max-w-lg mx-auto w-full">

          {/* Header */}
          <div className="flex items-center justify-between animate-slide-up">
            <div className="flex items-center gap-2.5">
              <div className="h-9 w-9 rounded-xl bg-emerald-400/10 border border-emerald-400/20 flex items-center justify-center">
                <Terminal className="h-4.5 w-4.5 text-emerald-400" />
              </div>
              <div>
                <h1 className="text-base font-semibold">Sandbox</h1>
                <p className="text-[10px] text-muted-foreground/50">Execute code securely</p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              {code && (
                <button
                  onClick={handleClear}
                  className="p-2 rounded-xl text-muted-foreground hover:text-foreground border border-border/50 transition-all"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
          </div>

          {/* Language selector */}
          <div className="relative animate-slide-up" style={{ animationDelay: "50ms" }}>
            <button
              onClick={() => setShowLangPicker(v => !v)}
              className="flex items-center gap-2.5 px-3.5 py-2.5 rounded-xl border border-border/50 bg-card/30 hover:bg-card/60 transition-all w-full text-left"
            >
              <Code2 className={cn("h-4 w-4", activeLang.color)} />
              <span className="text-sm font-medium flex-1">{activeLang.label}</span>
              <ChevronDown className={cn("h-3.5 w-3.5 text-muted-foreground/50 transition-transform", showLangPicker && "rotate-180")} />
            </button>

            {showLangPicker && (
              <div className="absolute top-full left-0 right-0 mt-1.5 rounded-xl border border-border/60 bg-card/95 backdrop-blur-xl shadow-xl z-10 overflow-hidden">
                {LANGUAGES.map(l => (
                  <button
                    key={l.id}
                    onClick={() => { setLanguage(l.id); setShowLangPicker(false); setCode(""); }}
                    className={cn(
                      "flex items-center gap-2.5 px-3.5 py-2.5 w-full text-left hover:bg-white/5 transition-colors",
                      l.id === language && "bg-white/[0.04]"
                    )}
                  >
                    <Code2 className={cn("h-3.5 w-3.5", l.color)} />
                    <span className="text-sm">{l.label}</span>
                    {l.id === language && <CheckCircle2 className="h-3.5 w-3.5 text-emerald-400 ml-auto" />}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Code Editor */}
          <div className="space-y-0 animate-slide-up" style={{ animationDelay: "100ms" }}>
            <div className="flex items-center justify-between px-3 py-1.5 rounded-t-xl bg-card/60 border border-b-0 border-border/50">
              <span className="text-[10px] font-mono text-muted-foreground/50">main{activeLang.id === "python" ? ".py" : activeLang.id === "typescript" ? ".ts" : activeLang.id === "bash" ? ".sh" : ".js"}</span>
              <button onClick={() => handleCopy(code)} className="p-1 text-muted-foreground/40 hover:text-muted-foreground transition-colors">
                <Copy className="h-3 w-3" />
              </button>
            </div>
            <textarea
              ref={textareaRef}
              value={code}
              onChange={e => setCode(e.target.value)}
              placeholder={activeLang.placeholder}
              spellCheck={false}
              className="w-full h-48 px-3.5 py-3 rounded-b-xl border border-border/50 bg-[#0a0c12] text-sm font-mono resize-none focus:outline-none focus:ring-1 focus:ring-emerald-400/30 text-emerald-50/90 placeholder:text-muted-foreground/30 leading-relaxed"
              onKeyDown={e => {
                if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
                  e.preventDefault();
                  handleRun();
                }
                if (e.key === "Tab") {
                  e.preventDefault();
                  const { selectionStart, selectionEnd } = e.currentTarget;
                  const newCode = code.slice(0, selectionStart) + "  " + code.slice(selectionEnd);
                  setCode(newCode);
                  requestAnimationFrame(() => {
                    e.currentTarget.selectionStart = e.currentTarget.selectionEnd = selectionStart + 2;
                  });
                }
              }}
            />
          </div>

          {/* Run button */}
          <button
            onClick={handleRun}
            disabled={exec.isPending || !code.trim()}
            className="w-full flex items-center justify-center gap-2.5 px-4 py-3 rounded-xl bg-emerald-500 hover:bg-emerald-400 disabled:opacity-40 disabled:cursor-not-allowed text-white font-medium transition-all animate-slide-up"
            style={{ animationDelay: "150ms" }}
          >
            {exec.isPending ? (
              <><Loader2 className="h-4 w-4 animate-spin" /><span>Running...</span></>
            ) : (
              <><Play className="h-4 w-4" /><span>Run Code</span><span className="text-emerald-200/50 text-xs ml-auto font-mono">⌘↵</span></>
            )}
          </button>

          {/* Output */}
          {hasOutput && result && (
            <div className="space-y-2 animate-slide-up">
              {/* Status bar */}
              <div className="flex items-center gap-2.5">
                {result.success ? (
                  <><CheckCircle2 className="h-3.5 w-3.5 text-emerald-400" /><span className="text-xs text-emerald-400 font-medium">Executed successfully</span></>
                ) : (
                  <><AlertCircle className="h-3.5 w-3.5 text-rose-400" /><span className="text-xs text-rose-400 font-medium">Exit {result.exit_code}</span></>
                )}
                <div className="flex items-center gap-1 ml-auto text-muted-foreground/40">
                  <Clock className="h-3 w-3" />
                  <span className="text-[10px] font-mono">{result.duration_ms}ms</span>
                </div>
                <button
                  onClick={() => setResult(null)}
                  className="p-1 text-muted-foreground/30 hover:text-muted-foreground/60 transition-colors"
                >
                  <X className="h-3 w-3" />
                </button>
              </div>

              {/* stdout */}
              {result.stdout && (
                <div className="space-y-0">
                  <div className="flex items-center justify-between px-3 py-1.5 rounded-t-xl bg-card/60 border border-b-0 border-border/50">
                    <span className="text-[10px] font-mono text-muted-foreground/50">stdout</span>
                    <button onClick={() => handleCopy(result.stdout)} className="p-1 text-muted-foreground/40 hover:text-muted-foreground transition-colors">
                      <Copy className="h-3 w-3" />
                    </button>
                  </div>
                  <pre className="rounded-b-xl border border-border/50 bg-[#0a0c12] px-3.5 py-3 text-xs font-mono text-emerald-300/90 overflow-x-auto max-h-64 whitespace-pre-wrap leading-relaxed">
                    {result.stdout}
                  </pre>
                </div>
              )}

              {/* stderr / compilation error */}
              {(result.stderr || result.compilation_error) && (
                <div className="space-y-0">
                  <div className="flex items-center justify-between px-3 py-1.5 rounded-t-xl bg-rose-900/20 border border-b-0 border-rose-500/20">
                    <span className="text-[10px] font-mono text-rose-400/70">
                      {result.compilation_error ? "compilation error" : "stderr"}
                    </span>
                  </div>
                  <pre className="rounded-b-xl border border-rose-500/20 bg-[#120a0a] px-3.5 py-3 text-xs font-mono text-rose-300/80 overflow-x-auto max-h-40 whitespace-pre-wrap leading-relaxed">
                    {result.compilation_error || result.stderr}
                  </pre>
                </div>
              )}

              {/* Timed out */}
              {result.timed_out && (
                <div className="px-3.5 py-2.5 rounded-xl border border-amber-500/20 bg-amber-500/5 text-xs text-amber-400 flex items-center gap-2">
                  <Clock className="h-3.5 w-3.5 shrink-0" />
                  Execution timed out (30s limit)
                </div>
              )}
            </div>
          )}

          {/* Empty state */}
          {!hasOutput && !exec.isPending && (
            <div className="flex flex-col items-center py-10 gap-3 text-center animate-fade-in">
              <div className="h-12 w-12 rounded-2xl bg-emerald-400/5 border border-emerald-400/15 flex items-center justify-center">
                <Terminal className="h-6 w-6 text-emerald-400/40" />
              </div>
              <p className="text-xs text-muted-foreground/40 max-w-48 leading-relaxed">
                Write code above and hit <span className="font-mono text-emerald-400/60">Run</span> — output appears here.
              </p>
            </div>
          )}
        </div>
      </div>
    </AppLayout>
  );
}
