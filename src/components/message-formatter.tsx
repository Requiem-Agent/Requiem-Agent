import React, { useState, useMemo } from "react";
import { Copy, Check, ChevronDown, ChevronUp, ExternalLink, AlertCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  LineChart, Line, BarChart, Bar, PieChart, Pie, Cell,
  XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer
} from "recharts";

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

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  async function handleCopy() {
    try { await navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 2000); } catch {}
  }
  return (
    <button onClick={handleCopy} className="flex items-center gap-1 px-2 py-1 rounded text-[10px] text-muted-foreground hover:text-foreground hover:bg-white/[0.06] transition-all">
      {copied ? <Check className="h-3 w-3 text-neutral-700" /> : <Copy className="h-3 w-3" />}
      {copied ? "تم النسخ" : "نسخ"}
    </button>
  );
}

function SvgBlock({ code }: { code: string }) {
  const [showSource, setShowSource] = useState(false);
  const safe = code.replace(/<script[\s\S]*?<\/script>/gi, "").replace(/on\w+="[^"]*"/gi, "");
  return (
    <div className="rounded-xl border border-border/50 overflow-hidden my-3 bg-white/[0.02]">
      <div className="flex items-center justify-between px-3 py-1.5 bg-card border-b border-border/40">
        <span className="text-[10px] font-mono text-neutral-500 uppercase tracking-wider">معاينة SVG</span>
        <div className="flex gap-1">
          <button onClick={() => setShowSource(s => !s)} className="text-[10px] text-muted-foreground px-2 py-1 rounded hover:bg-white/[0.06]">{showSource ? "معاينة" : "المصدر"}</button>
          <CopyButton text={code} />
        </div>
      </div>
      {showSource
        ? <pre className="code-block text-[0.75rem] p-3 text-[#e2e8f0] overflow-x-auto"><code>{code}</code></pre>
        : <div className="p-4 flex items-center justify-center overflow-auto" dangerouslySetInnerHTML={{ __html: safe }} />
      }
    </div>
  );
}

function HtmlPreviewBlock({ code }: { code: string }) {
  const [showPreview, setShowPreview] = useState(true);
  const blobUrl = useMemo(() => {
    if (typeof URL === "undefined") return "";
    const b = new Blob([code], { type: "text/html" });
    return URL.createObjectURL(b);
  }, [code]);
  return (
    <div className="rounded-xl border border-border/50 overflow-hidden my-3">
      <div className="flex items-center justify-between px-3 py-1.5 bg-card border-b border-border/40">
        <span className="text-[10px] font-mono text-neutral-500 uppercase">معاينة HTML</span>
        <div className="flex gap-1">
          <button onClick={() => setShowPreview(p => !p)} className="text-[10px] text-muted-foreground px-2 py-1 rounded hover:bg-white/[0.06]">{showPreview ? "المصدر" : "معاينة"}</button>
          <CopyButton text={code} />
        </div>
      </div>
      {showPreview
        ? <iframe src={blobUrl} className="w-full h-64 border-0 bg-white" sandbox="allow-scripts" title="معاينة HTML" />
        : <pre className="code-block text-[0.75rem] p-3 text-[#e2e8f0] overflow-x-auto max-h-64"><code>{code}</code></pre>
      }
    </div>
  );
}

const CHART_COLORS = ["#6366f1","#22d3ee","#10b981","#f59e0b","#ef4444","#8b5cf6"];
function ChartBlock({ code }: { code: string }) {
  const parsed = useMemo(() => { try { return JSON.parse(code); } catch { return null; } }, [code]);
  if (!parsed) return (
    <div className="rounded-xl border border-neutral-300 bg-neutral-50 p-3 my-3 flex items-center gap-2 text-xs text-neutral-500">
      <AlertCircle className="h-4 w-4 shrink-0" />بيانات الرسم البياني غير صالحة
    </div>
  );
  const { type="bar", data=[], title, xKey="name", bars=[], lines=[], dataKey="value" } = parsed;
  const keys = bars.length ? bars : lines.length ? lines : [dataKey];
  return (
    <div className="rounded-xl border border-border/50 overflow-hidden my-3">
      <div className="flex items-center justify-between px-3 py-1.5 bg-card border-b border-border/40">
        <span className="text-[10px] font-mono text-neutral-500 uppercase">{type === "bar" ? "أعمدة" : type === "line" ? "خطي" : type === "pie" ? "دائري" : type} — رسم بياني{title ? ` · ${title}` : ""}</span>
        <CopyButton text={code} />
      </div>
      <div className="p-3 bg-neutral-950">
        <ResponsiveContainer width="100%" height={220}>
          {type === "line" ? (
            <LineChart data={data}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e2028" />
              <XAxis dataKey={xKey} tick={{ fill:"#9ca3af", fontSize:10 }} />
              <YAxis tick={{ fill:"#9ca3af", fontSize:10 }} />
              <Tooltip contentStyle={{ background:"#13141a", border:"1px solid #2d2e3a", borderRadius:8, fontSize:11 }} />
              <Legend />
              {keys.map((k: string, i: number) => <Line key={k} type="monotone" dataKey={k} stroke={CHART_COLORS[i%6]} strokeWidth={2} dot={false} />)}
            </LineChart>
          ) : type === "pie" ? (
            <PieChart>
              <Pie data={data} dataKey={dataKey} nameKey={xKey} cx="50%" cy="50%" outerRadius={80} label>
                {data.map((_: unknown, i: number) => <Cell key={i} fill={CHART_COLORS[i%6]} />)}
              </Pie>
              <Tooltip contentStyle={{ background:"#13141a", border:"1px solid #2d2e3a", borderRadius:8, fontSize:11 }} />
              <Legend />
            </PieChart>
          ) : (
            <BarChart data={data}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e2028" />
              <XAxis dataKey={xKey} tick={{ fill:"#9ca3af", fontSize:10 }} />
              <YAxis tick={{ fill:"#9ca3af", fontSize:10 }} />
              <Tooltip contentStyle={{ background:"#13141a", border:"1px solid #2d2e3a", borderRadius:8, fontSize:11 }} />
              <Legend />
              {keys.map((k: string, i: number) => <Bar key={k} dataKey={k} fill={CHART_COLORS[i%6]} radius={[3,3,0,0]} />)}
            </BarChart>
          )}
        </ResponsiveContainer>
      </div>
    </div>
  );
}

function MermaidBlock({ code }: { code: string }) {
  return (
    <div className="rounded-xl border border-primary/20 bg-primary/[0.03] overflow-hidden my-3">
      <div className="flex items-center justify-between px-3 py-1.5 bg-primary/5 border-b border-primary/15">
        <span className="text-[10px] font-mono text-primary/60 uppercase tracking-wider">مخطط</span>
        <CopyButton text={code} />
      </div>
      <pre className="text-[0.78rem] p-3 text-foreground/70 whitespace-pre leading-relaxed font-mono overflow-x-auto">{code}</pre>
    </div>
  );
}

function ImageBlock({ src, alt }: { src: string; alt: string }) {
  const [error, setError] = useState(false);
  if (error) return <span className="text-xs text-muted-foreground/50">[صورة: {alt}]</span>;
  return (
    <div className="my-3 rounded-xl overflow-hidden border border-border/50 inline-block max-w-full">
      <img src={src} alt={alt} className="max-w-full max-h-80 object-contain" onError={() => setError(true)} />
    </div>
  );
}

function TableBlock({ rows }: { rows: string[][] }) {
  if (rows.length === 0) return null;
  const [header, ...body] = rows;
  return (
    <div className="my-3 overflow-x-auto rounded-xl border border-border/50">
      <table className="min-w-full text-xs">
        <thead>
          <tr className="bg-card/60 border-b border-border/40">
            {header.map((cell, i) => <th key={i} className="px-3 py-2 text-left font-semibold text-foreground/80 whitespace-nowrap">{renderInline(cell.trim())}</th>)}
          </tr>
        </thead>
        <tbody>
          {body.map((row, ri) => (
            <tr key={ri} className={cn("border-b border-border/20", ri%2===1 && "bg-white/[0.01]")}>
              {row.map((cell, ci) => <td key={ci} className="px-3 py-2 text-foreground/70 whitespace-nowrap">{renderInline(cell.trim())}</td>)}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function InlineCode({ children }: { children: React.ReactNode }) {
  return <code className="px-1.5 py-0.5 rounded bg-neutral-950 border border-border/40 text-neutral-700 font-mono text-[0.8em]">{children}</code>;
}

function renderInline(text: string): React.ReactNode {
  const parts = text.split(/(!\[[^\]]*\]\([^)]+\)|\[[^\]]+\]\([^)]+\)|`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*|__[^_]+__)/g);
  return parts.map((part, i) => {
    if (!part) return null;
    if (/^!\[/.test(part)) { const m=part.match(/^!\[([^\]]*)\]\(([^)]+)\)$/); if(m) return <ImageBlock key={i} alt={m[1]} src={m[2]} />; }
    if (/^\[/.test(part)) { const m=part.match(/^\[([^\]]+)\]\(([^)]+)\)$/); if(m) return <a key={i} href={m[2]} target="_blank" rel="noopener noreferrer" className="text-primary/80 hover:text-primary underline underline-offset-2 inline-flex items-center gap-0.5">{m[1]}<ExternalLink className="h-2.5 w-2.5"/></a>; }
    if (part.startsWith("`")&&part.endsWith("`")) return <InlineCode key={i}>{part.slice(1,-1)}</InlineCode>;
    if ((part.startsWith("**")&&part.endsWith("**"))||(part.startsWith("__")&&part.endsWith("__"))) return <strong key={i} className="font-semibold text-foreground">{part.slice(2,-2)}</strong>;
    if (part.startsWith("*")&&part.endsWith("*")) return <em key={i} className="italic text-foreground/80">{part.slice(1,-1)}</em>;
    return part;
  });
}

function renderMarkdown(text: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let key = 0;
  const lines = text.split("\n");
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (/^# (.+)/.test(line)) { nodes.push(<h1 key={key++} className="text-lg font-bold mt-4 mb-2 text-foreground">{renderInline(line.slice(2))}</h1>); i++; continue; }
    if (/^## (.+)/.test(line)) { nodes.push(<h2 key={key++} className="text-base font-semibold mt-3 mb-1.5 text-foreground">{renderInline(line.slice(3))}</h2>); i++; continue; }
    if (/^### (.+)/.test(line)) { nodes.push(<h3 key={key++} className="text-sm font-semibold mt-2 mb-1 text-foreground/90">{renderInline(line.slice(4))}</h3>); i++; continue; }
    if (/^#### (.+)/.test(line)) { nodes.push(<h4 key={key++} className="text-xs font-semibold mt-2 mb-1 text-foreground/80 uppercase tracking-wide">{renderInline(line.slice(5))}</h4>); i++; continue; }
    if (/^---+$/.test(line.trim())) { nodes.push(<hr key={key++} className="border-border/40 my-3" />); i++; continue; }
    if (/^\|.+\|/.test(line)) {
      const tableRows: string[][] = [];
      while (i < lines.length && /^\|.+\|/.test(lines[i])) {
        const row = lines[i].split("|").slice(1,-1);
        if (!row.every(c => /^[-: ]+$/.test(c.trim()))) tableRows.push(row);
        i++;
      }
      if (tableRows.length > 0) nodes.push(<TableBlock key={key++} rows={tableRows} />);
      continue;
    }
    if (/^[-*•] (.+)/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*•] (.+)/.test(lines[i])) { items.push(lines[i].replace(/^[-*•] /, "")); i++; }
      nodes.push(<ul key={key++} className="space-y-1 my-2 pl-1">{items.map((item, j) => <li key={j} className="flex gap-2 text-sm text-foreground/85 leading-relaxed"><span className="text-primary/60 mt-0.5 shrink-0">›</span><span>{renderInline(item)}</span></li>)}</ul>);
      continue;
    }
    if (/^\d+\. (.+)/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+\. (.+)/.test(lines[i])) { items.push(lines[i].replace(/^\d+\. /, "")); i++; }
      nodes.push(<ol key={key++} className="space-y-1 my-2 pl-1 list-none">{items.map((item, j) => <li key={j} className="flex gap-2.5 text-sm text-foreground/85 leading-relaxed"><span className="text-primary/60 font-mono text-xs mt-0.5 shrink-0 w-4">{j+1}.</span><span>{renderInline(item)}</span></li>)}</ol>);
      continue;
    }
    if (/^> (.+)/.test(line)) { nodes.push(<blockquote key={key++} className="border-l-2 border-primary/40 pl-3 my-2 text-sm text-muted-foreground italic">{renderInline(line.slice(2))}</blockquote>); i++; continue; }
    if (line.trim() === "") { nodes.push(<div key={key++} className="h-2" />); i++; continue; }
    if (line.trim()) nodes.push(<p key={key++} className="text-sm text-foreground/90 leading-relaxed">{renderInline(line)}</p>);
    i++;
  }
  return nodes;
}

// Language → color accent for code block border
const LANG_ACCENT: Record<string, string> = {
  rust: "border-l-neutral-600",
  rs: "border-l-neutral-600",
  typescript: "border-l-neutral-500",
  tsx: "border-l-neutral-500",
  ts: "border-l-neutral-500",
  javascript: "border-l-neutral-500",
  js: "border-l-neutral-500",
  jsx: "border-l-neutral-500",
  python: "border-l-neutral-400",
  py: "border-l-neutral-400",
  bash: "border-l-neutral-500",
  sh: "border-l-neutral-500",
  shell: "border-l-neutral-500",
  json: "border-l-neutral-500",
  sql: "border-l-neutral-500",
  toml: "border-l-neutral-500",
  yaml: "border-l-neutral-500",
  html: "border-l-neutral-500",
  css: "border-l-neutral-400",
};

// Language → badge label color
const LANG_COLOR: Record<string, string> = {
  rust: "text-neutral-600", rs: "text-neutral-600",
  typescript: "text-neutral-600", tsx: "text-neutral-600", ts: "text-neutral-600",
  javascript: "text-neutral-600", js: "text-neutral-600",
  python: "text-neutral-600", py: "text-neutral-600",
  bash: "text-neutral-700", sh: "text-neutral-700", shell: "text-neutral-700",
  json: "text-neutral-600",
  sql: "text-neutral-600",
  html: "text-neutral-600",
  css: "text-neutral-500",
};

function CodeBlock({ language, code }: { language: string; code: string }) {
  const [collapsed, setCollapsed] = useState(false);
  const lines = code.split("\n");
  const tooLong = lines.length > 25;
  const lang = language.toLowerCase();

  if (lang === "svg" || (!lang && code.trimStart().startsWith("<svg"))) return <SvgBlock code={code} />;
  if (lang === "html" && code.includes("<body")) return <HtmlPreviewBlock code={code} />;
  if (lang === "chart" || lang === "recharts") return <ChartBlock code={code} />;
  if (lang === "mermaid") return <MermaidBlock code={code} />;

  const accentClass = LANG_ACCENT[lang] ?? "border-l-border/40";
  const labelColor  = LANG_COLOR[lang]  ?? "text-muted-foreground/60";
  const displayLang = language || "text";

  return (
    <div className={cn("rounded-xl bg-neutral-950 border border-border/50 border-l-2 overflow-hidden my-3 shadow-lg", accentClass)}>
      {/* Header bar */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-neutral-950 border-b border-border/30">
        <div className="flex items-center gap-2.5">
          {/* macOS traffic lights */}
          <div className="flex gap-1.5 shrink-0">
            <span className="h-2.5 w-2.5 rounded-full bg-neutral-500"/>
            <span className="h-2.5 w-2.5 rounded-full bg-neutral-500"/>
            <span className="h-2.5 w-2.5 rounded-full bg-neutral-700"/>
          </div>
          <span className={cn("text-[10px] font-mono uppercase tracking-wider font-semibold", labelColor)}>
            {displayLang}
          </span>
          <span className="text-[10px] font-mono text-muted-foreground/30">
            {lines.length}L
          </span>
        </div>
        <div className="flex items-center gap-1">
          {tooLong && (
            <button
              onClick={() => setCollapsed(c => !c)}
              className="flex items-center gap-1 px-2 py-0.5 rounded text-[10px] text-muted-foreground/60 hover:text-foreground hover:bg-white/[0.06] transition-all border border-transparent hover:border-border/40"
            >
              {collapsed
                ? <><ChevronDown className="h-3 w-3"/>توسيع</>
                : <><ChevronUp className="h-3 w-3"/>طيّ</>
              }
            </button>
          )}
          <CopyButton text={code} />
        </div>
      </div>

      {/* Code body */}
      {!collapsed ? (
        <div className="overflow-x-auto">
          <pre className="code-block text-[0.8125rem] p-4 leading-relaxed">
            <code className="text-[#e2e8f0]">{code}</code>
          </pre>
        </div>
      ) : (
        <button
          onClick={() => setCollapsed(false)}
          className="w-full px-4 py-2 text-xs text-muted-foreground/40 font-mono hover:text-muted-foreground/70 hover:bg-white/[0.02] transition-all text-left"
        >
          ▸ {lines.length} سطر مخفي — انقر للتوسيع
        </button>
      )}
    </div>
  );
}

// ── Think block — collapsible chain-of-thought ────────────────────────────────
function ThinkBlock({ content }: { content: string }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="my-2 rounded-xl border border-neutral-300 bg-neutral-50 overflow-hidden">
      <button
        onClick={() => setOpen(o => !o)}
        className="w-full flex items-center gap-2 px-3 py-2 text-xs hover:bg-white/[0.02] transition-colors text-left"
      >
        <span className="text-neutral-600 text-[10px] font-mono">⟨think⟩</span>
        <span className="text-neutral-500 text-[10px]">
          {open ? "إخفاء سلسلة التفكير" : "إظهار سلسلة التفكير"}
        </span>
        <span className="ml-auto text-muted-foreground/30">
          {open ? "▲" : "▼"}
        </span>
      </button>
      {open && (
        <div className="px-3 pb-3 pt-1 text-[0.75rem] text-neutral-500 leading-relaxed italic border-t border-neutral-200 font-mono whitespace-pre-wrap">
          {content.trim()}
        </div>
      )}
    </div>
  );
}

export function FormattedMessage({ content }: { content: string }) {
  // Strip residual JSON wrapping before rendering
  let cleaned = content;
  const isBareJson = cleaned.trim().startsWith("{") || cleaned.trim().startsWith("[");
  if (isBareJson) {
    try {
      const parsed = JSON.parse(cleaned.trim());
      const extracted =
        parsed?.choices?.[0]?.message?.content ||
        parsed?.choices?.[0]?.delta?.content ||
        parsed?.response || parsed?.text || parsed?.content ||
        (typeof parsed?.message === "string" && !parsed?.error ? parsed.message : null);
      if (extracted && typeof extracted === "string") cleaned = extracted;
      else return null;
    } catch { /* not JSON */ }
  }

  const normalized = cleaned
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n")
    .replace(/\t/g, "  ")
    .trim();

  if (!normalized) return null;

  

  // Step 3: extract <think>...</think> blocks first (case-insensitive, multiline)
  // Also handle [think]...[/think] variant that some models produce
  const withThink = normalized.split(/(<think>[\s\S]*?<\/think>|\[think\][\s\S]*?\[\/think\])/gi);

  // Step 4: for non-think segments, split by code fences
  return (
    <div className="space-y-1 min-w-0">
      {withThink.map((seg, i) => {
        // Think block (<think> or [think] variant)
        if (/^<think>/i.test(seg) && /<\/think>$/i.test(seg)) {
          const inner = seg.replace(/^<think>/i, "").replace(/<\/think>$/i, "");
          return <ThinkBlock key={i} content={inner} />;
        }
        if (/^\[think\]/i.test(seg) && /\[\/think\]$/i.test(seg)) {
          const inner = seg.replace(/^\[think\]/i, "").replace(/\[\/think\]$/i, "");
          return <ThinkBlock key={i} content={inner} />;
        }

        // Code fences within segment — handle nested ``` correctly
        const codeSplit = seg.split(/(```[\w]*\n[\s\S]*?```|```[\s\S]*?```)/g);
        return (
          <React.Fragment key={i}>
            {codeSplit.map((part, j) => {
              if (part.startsWith("```")) {
                // Strip outer fences
                const inner = part.slice(3).replace(/```$/, "");
                const firstBreak = inner.indexOf("\n");
                const lang = firstBreak > -1 ? inner.slice(0, firstBreak).trim().toLowerCase() : "";
                const code = firstBreak > -1 ? inner.slice(firstBreak + 1) : inner;
                const trimmedCode = code.replace(/\n$/, ""); // trim trailing newline only
                if (!lang && trimmedCode.trimStart().startsWith("<svg")) return <SvgBlock key={j} code={trimmedCode} />;
                return <CodeBlock key={j} language={lang} code={trimmedCode} />;
              }
              if (!part.trim()) return null;
              return <React.Fragment key={j}>{renderMarkdown(part)}</React.Fragment>;
            })}
          </React.Fragment>
        );
      })}
    </div>
  );
}
