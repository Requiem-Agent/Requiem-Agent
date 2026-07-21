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
      {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
      {copied ? "copied" : "copy"}
    </button>
  );
}

function SvgBlock({ code }: { code: string }) {
  const [showSource, setShowSource] = useState(false);
  const safe = code.replace(/<script[\s\S]*?<\/script>/gi, "").replace(/on\w+="[^"]*"/gi, "");
  return (
    <div className="rounded-xl border border-border/50 overflow-hidden my-3 bg-white/[0.02]">
      <div className="flex items-center justify-between px-3 py-1.5 bg-card border-b border-border/40">
        <span className="text-[10px] font-mono text-emerald-400/70 uppercase tracking-wider">SVG Preview</span>
        <div className="flex gap-1">
          <button onClick={() => setShowSource(s => !s)} className="text-[10px] text-muted-foreground px-2 py-1 rounded hover:bg-white/[0.06]">{showSource ? "Preview" : "Source"}</button>
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
        <span className="text-[10px] font-mono text-cyan-400/70 uppercase">HTML Preview</span>
        <div className="flex gap-1">
          <button onClick={() => setShowPreview(p => !p)} className="text-[10px] text-muted-foreground px-2 py-1 rounded hover:bg-white/[0.06]">{showPreview ? "Source" : "Preview"}</button>
          <CopyButton text={code} />
        </div>
      </div>
      {showPreview
        ? <iframe src={blobUrl} className="w-full h-64 border-0 bg-white" sandbox="allow-scripts" title="HTML preview" />
        : <pre className="code-block text-[0.75rem] p-3 text-[#e2e8f0] overflow-x-auto max-h-64"><code>{code}</code></pre>
      }
    </div>
  );
}

const CHART_COLORS = ["#6366f1","#22d3ee","#10b981","#f59e0b","#ef4444","#8b5cf6"];
function ChartBlock({ code }: { code: string }) {
  const parsed = useMemo(() => { try { return JSON.parse(code); } catch { return null; } }, [code]);
  if (!parsed) return (
    <div className="rounded-xl border border-rose-500/20 bg-rose-500/5 p-3 my-3 flex items-center gap-2 text-xs text-rose-400">
      <AlertCircle className="h-4 w-4 shrink-0" />Invalid chart JSON
    </div>
  );
  const { type="bar", data=[], title, xKey="name", bars=[], lines=[], dataKey="value" } = parsed;
  const keys = bars.length ? bars : lines.length ? lines : [dataKey];
  return (
    <div className="rounded-xl border border-border/50 overflow-hidden my-3">
      <div className="flex items-center justify-between px-3 py-1.5 bg-card border-b border-border/40">
        <span className="text-[10px] font-mono text-amber-400/70 uppercase">{type} chart{title ? ` · ${title}` : ""}</span>
        <CopyButton text={code} />
      </div>
      <div className="p-3 bg-[#0a0b0e]">
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
        <span className="text-[10px] font-mono text-primary/60 uppercase tracking-wider">Diagram</span>
        <CopyButton text={code} />
      </div>
      <pre className="text-[0.78rem] p-3 text-foreground/70 whitespace-pre leading-relaxed font-mono overflow-x-auto">{code}</pre>
    </div>
  );
}

function ImageBlock({ src, alt }: { src: string; alt: string }) {
  const [error, setError] = useState(false);
  if (error) return <span className="text-xs text-muted-foreground/50">[Image: {alt}]</span>;
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
  return <code className="px-1.5 py-0.5 rounded bg-[#0f1014] border border-border/40 text-cyan-300 font-mono text-[0.8em]">{children}</code>;
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

function CodeBlock({ language, code }: { language: string; code: string }) {
  const [collapsed, setCollapsed] = useState(false);
  const lines = code.split("\n");
  const tooLong = lines.length > 30;
  const lang = language.toLowerCase();
  if (lang === "svg" || (!lang && code.trimStart().startsWith("<svg"))) return <SvgBlock code={code} />;
  if (lang === "html" && code.includes("<body")) return <HtmlPreviewBlock code={code} />;
  if (lang === "chart" || lang === "recharts") return <ChartBlock code={code} />;
  if (lang === "mermaid") return <MermaidBlock code={code} />;
  return (
    <div className="rounded-xl bg-[#0a0b0e] border border-border/50 overflow-hidden my-3 shadow-md">
      <div className="flex items-center justify-between px-3 py-1.5 bg-[#0f1014] border-b border-border/40">
        <div className="flex items-center gap-2">
          <div className="flex gap-1.5"><span className="h-2.5 w-2.5 rounded-full bg-rose-500/60"/><span className="h-2.5 w-2.5 rounded-full bg-amber-500/60"/><span className="h-2.5 w-2.5 rounded-full bg-emerald-500/60"/></div>
          {language && <span className="text-[10px] font-mono text-muted-foreground/60 uppercase tracking-wider">{language}</span>}
          {tooLong && <span className="text-[10px] font-mono text-muted-foreground/40">{lines.length} lines</span>}
        </div>
        <div className="flex items-center gap-1">
          {tooLong && <button onClick={() => setCollapsed(c=>!c)} className="flex items-center gap-1 px-2 py-1 rounded text-[10px] text-muted-foreground hover:text-foreground hover:bg-white/[0.06] transition-all">{collapsed ? <ChevronDown className="h-3 w-3"/> : <ChevronUp className="h-3 w-3"/>}{collapsed ? "expand" : "collapse"}</button>}
          <CopyButton text={code} />
        </div>
      </div>
      {!collapsed ? <div className="overflow-x-auto"><pre className="code-block text-[0.8125rem] p-4 leading-relaxed"><code className="text-[#e2e8f0]">{code}</code></pre></div>
        : <div className="px-4 py-2 text-xs text-muted-foreground/40 font-mono">{lines.length} lines hidden · click expand</div>}
    </div>
  );
}

export function FormattedMessage({ content }: { content: string }) {
  const normalized = content.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "");
  const segments = normalized.split(/(```[\s\S]*?```)/g);
  return (
    <div className="space-y-1">
      {segments.map((seg, i) => {
        if (seg.startsWith("```") && seg.endsWith("```")) {
          const inner = seg.slice(3, -3);
          const firstBreak = inner.indexOf("\n");
          const lang = firstBreak > -1 ? inner.slice(0, firstBreak).trim() : "";
          const code = firstBreak > -1 ? inner.slice(firstBreak + 1) : inner;
          if (!lang && code.trimStart().startsWith("<svg")) return <SvgBlock key={i} code={code.trimEnd()} />;
          return <CodeBlock key={i} language={lang} code={code.trimEnd()} />;
        }
        if (!seg.trim()) return null;
        return <React.Fragment key={i}>{renderMarkdown(seg)}</React.Fragment>;
      })}
    </div>
  );
}
