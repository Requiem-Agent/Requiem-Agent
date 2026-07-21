import React, { useState } from "react";
import { Copy, Check, ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "@/lib/utils";

// ── Typewriter text ────────────────────────────────────────────────────────────
export function TypewriterText({ text, speed = 8 }: { text: string; speed?: number }) {
  const [displayed, setDisplayed] = React.useState("");
  React.useEffect(() => {
    let i = 0;
    const id = setInterval(() => {
      setDisplayed(text.slice(0, i));
      i++;
      if (i > text.length) clearInterval(id);
    }, speed);
    return () => clearInterval(id);
  }, [text, speed]);
  return <span>{displayed}</span>;
}

// ── Copy button ────────────────────────────────────────────────────────────────
function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {}
  }
  return (
    <button
      onClick={handleCopy}
      className="flex items-center gap-1 px-2 py-1 rounded text-[10px] text-muted-foreground hover:text-foreground hover:bg-white/[0.06] transition-all"
      title="Copy"
    >
      {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
      {copied ? "copied" : "copy"}
    </button>
  );
}

// ── Code block ─────────────────────────────────────────────────────────────────
function CodeBlock({ language, code }: { language: string; code: string }) {
  const [collapsed, setCollapsed] = useState(false);
  const lines = code.split("\n");
  const tooLong = lines.length > 30;

  return (
    <div className="rounded-xl bg-[#0a0b0e] border border-border/50 overflow-hidden my-3 shadow-md">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-[#0f1014] border-b border-border/40">
        <div className="flex items-center gap-2">
          {/* Dots */}
          <div className="flex gap-1.5">
            <span className="h-2.5 w-2.5 rounded-full bg-rose-500/60" />
            <span className="h-2.5 w-2.5 rounded-full bg-amber-500/60" />
            <span className="h-2.5 w-2.5 rounded-full bg-emerald-500/60" />
          </div>
          {language && (
            <span className="text-[10px] font-mono text-muted-foreground/60 uppercase tracking-wider">
              {language}
            </span>
          )}
          {tooLong && (
            <span className="text-[10px] font-mono text-muted-foreground/40">
              {lines.length} lines
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {tooLong && (
            <button
              onClick={() => setCollapsed(c => !c)}
              className="flex items-center gap-1 px-2 py-1 rounded text-[10px] text-muted-foreground hover:text-foreground hover:bg-white/[0.06] transition-all"
            >
              {collapsed ? <ChevronDown className="h-3 w-3" /> : <ChevronUp className="h-3 w-3" />}
              {collapsed ? "expand" : "collapse"}
            </button>
          )}
          <CopyButton text={code} />
        </div>
      </div>

      {/* Code */}
      {!collapsed && (
        <div className="overflow-x-auto">
          <pre className="code-block text-[0.8125rem] p-4 leading-relaxed">
            <code className="text-[#e2e8f0]">{code}</code>
          </pre>
        </div>
      )}
      {collapsed && (
        <div className="px-4 py-2 text-xs text-muted-foreground/40 font-mono">
          {lines.length} lines hidden · click expand to view
        </div>
      )}
    </div>
  );
}

// ── Inline code ────────────────────────────────────────────────────────────────
function InlineCode({ children }: { children: React.ReactNode }) {
  return (
    <code className="px-1.5 py-0.5 rounded bg-[#0f1014] border border-border/40 text-cyan-300 font-mono text-[0.8em]">
      {children}
    </code>
  );
}

// ── Markdown renderer ─────────────────────────────────────────────────────────
function renderMarkdown(text: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let key = 0;

  // Split into lines
  const lines = text.split("\n");
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // H1
    if (/^# (.+)/.test(line)) {
      nodes.push(<h1 key={key++} className="text-lg font-bold mt-4 mb-2 text-foreground gradient-text">{line.slice(2)}</h1>);
      i++; continue;
    }
    // H2
    if (/^## (.+)/.test(line)) {
      nodes.push(<h2 key={key++} className="text-base font-semibold mt-3 mb-1.5 text-foreground">{line.slice(3)}</h2>);
      i++; continue;
    }
    // H3
    if (/^### (.+)/.test(line)) {
      nodes.push(<h3 key={key++} className="text-sm font-semibold mt-2 mb-1 text-foreground/90">{line.slice(4)}</h3>);
      i++; continue;
    }
    // HR
    if (/^---+$/.test(line.trim())) {
      nodes.push(<hr key={key++} className="border-border/40 my-3" />);
      i++; continue;
    }
    // Bullet list
    if (/^[-*•] (.+)/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*•] (.+)/.test(lines[i])) {
        items.push(lines[i].replace(/^[-*•] /, ""));
        i++;
      }
      nodes.push(
        <ul key={key++} className="space-y-1 my-2 pl-1">
          {items.map((item, j) => (
            <li key={j} className="flex gap-2 text-sm text-foreground/85 leading-relaxed">
              <span className="text-primary/60 mt-0.5 shrink-0">›</span>
              <span>{renderInline(item)}</span>
            </li>
          ))}
        </ul>
      );
      continue;
    }
    // Numbered list
    if (/^\d+\. (.+)/.test(line)) {
      const items: string[] = [];
      let n = 1;
      while (i < lines.length && /^\d+\. (.+)/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\. /, ""));
        i++;
      }
      nodes.push(
        <ol key={key++} className="space-y-1 my-2 pl-1 list-none">
          {items.map((item, j) => (
            <li key={j} className="flex gap-2.5 text-sm text-foreground/85 leading-relaxed">
              <span className="text-primary/60 font-mono text-xs mt-0.5 shrink-0 w-4">{j + 1}.</span>
              <span>{renderInline(item)}</span>
            </li>
          ))}
        </ol>
      );
      continue;
    }
    // Blockquote
    if (/^> (.+)/.test(line)) {
      nodes.push(
        <blockquote key={key++} className="border-l-2 border-primary/40 pl-3 my-2 text-sm text-muted-foreground italic">
          {line.slice(2)}
        </blockquote>
      );
      i++; continue;
    }
    // Empty line → spacing
    if (line.trim() === "") {
      nodes.push(<div key={key++} className="h-2" />);
      i++; continue;
    }
    // Normal paragraph
    if (line.trim()) {
      nodes.push(
        <p key={key++} className="text-sm text-foreground/90 leading-relaxed">
          {renderInline(line)}
        </p>
      );
    }
    i++;
  }
  return nodes;
}

// ── Inline markdown (bold, italic, code) ─────────────────────────────────────
function renderInline(text: string): React.ReactNode {
  const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*|__[^_]+__)/g);
  return parts.map((part, i) => {
    if (part.startsWith("`") && part.endsWith("`"))
      return <InlineCode key={i}>{part.slice(1, -1)}</InlineCode>;
    if ((part.startsWith("**") && part.endsWith("**")) || (part.startsWith("__") && part.endsWith("__")))
      return <strong key={i} className="font-semibold text-foreground">{part.slice(2, -2)}</strong>;
    if (part.startsWith("*") && part.endsWith("*"))
      return <em key={i} className="italic text-foreground/80">{part.slice(1, -1)}</em>;
    return part;
  });
}

// ── Main FormattedMessage ─────────────────────────────────────────────────────
export function FormattedMessage({ content }: { content: string }) {
  // Normalize escaped sequences that may have been stored literally in DB
  const normalized = content
    .replace(/\\n/g, "\n")
    .replace(/\\t/g, "\t")
    .replace(/\\r/g, "");

  // Split by code fences first
  const segments = normalized.split(/(```[\s\S]*?```)/g);

  return (
    <div className="space-y-1">
      {segments.map((seg, i) => {
        if (seg.startsWith("```") && seg.endsWith("```")) {
          const inner = seg.slice(3, -3);
          const firstBreak = inner.indexOf("\n");
          const lang = firstBreak > -1 ? inner.slice(0, firstBreak).trim() : "";
          const code = firstBreak > -1 ? inner.slice(firstBreak + 1) : inner;
          return <CodeBlock key={i} language={lang} code={code.trimEnd()} />;
        }
        // Render markdown for non-code segments
        if (!seg.trim()) return null;
        return <React.Fragment key={i}>{renderMarkdown(seg)}</React.Fragment>;
      })}
    </div>
  );
}
