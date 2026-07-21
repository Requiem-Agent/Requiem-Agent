import { useState } from "react";
import { AppLayout } from "@/components/layout";
import { useBots, useBotMutations } from "@/hooks/use-bots";
import { useToast } from "@/hooks/use-toast";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import * as z from "zod";
import {
  Plus, Bot, Rocket, Trash2, Loader2, ExternalLink,
  CheckCircle2, AlertCircle, Clock, Zap, RefreshCw, X,
  MessageSquare, Settings2, Globe, Key, Copy, Check,
  ChevronRight, Terminal, ArrowRight, Sparkles, Shield,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Form, FormField, FormItem, FormLabel, FormControl, FormDescription, FormMessage } from "@/components/ui/form";
import { cn } from "@/lib/utils";

// ── Schema ─────────────────────────────────────────────────────────────────────
const botSchema = z.object({
  name:        z.string().min(2, "At least 2 characters"),
  token:       z.string().min(10, "Paste your bot token from @BotFather"),
  description: z.string().optional(),
  purpose:     z.string().optional(),
});
type BotFormData = z.infer<typeof botSchema>;

// ── Status map ─────────────────────────────────────────────────────────────────
const STATUS_META: Record<string, { label: string; color: string; bg: string; border: string; Icon: React.ElementType }> = {
  active:    { label: "active",    color: "text-emerald-400", bg: "bg-emerald-400/10", border: "border-emerald-400/25", Icon: CheckCircle2 },
  deploying: { label: "deploying", color: "text-amber-400",   bg: "bg-amber-400/10",   border: "border-amber-400/25",   Icon: RefreshCw    },
  building:  { label: "building",  color: "text-cyan-400",    bg: "bg-cyan-400/10",    border: "border-cyan-400/25",    Icon: Zap          },
  deployed:  { label: "deployed",  color: "text-cyan-400",    bg: "bg-cyan-400/10",    border: "border-cyan-400/25",    Icon: CheckCircle2 },
  error:     { label: "error",     color: "text-rose-400",    bg: "bg-rose-400/10",    border: "border-rose-400/25",    Icon: AlertCircle  },
  pending:   { label: "pending",   color: "text-muted-foreground", bg: "bg-muted/40",  border: "border-border",         Icon: Clock        },
  sleeping:  { label: "sleeping",  color: "text-muted-foreground", bg: "bg-muted/20",  border: "border-border/50",      Icon: Clock        },
};

// ── Step-by-step guide to get a Telegram Bot Token via Managed Bots ───────────
function ManagedBotGuide() {
  const [step, setStep] = useState(0);
  const [copied, setCopied] = useState(false);

  const steps = [
    {
      title: "Open @BotFather",
      desc: "Start a chat with @BotFather in Telegram",
      action: (
        <a href="https://t.me/BotFather" target="_blank" rel="noopener noreferrer"
          className="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-primary/10 text-primary border border-primary/25 text-xs font-medium hover:bg-primary/20 transition-all">
          <ArrowRight className="h-3.5 w-3.5" />Open @BotFather
        </a>
      )
    },
    {
      title: "Create a new bot",
      desc: 'Send /newbot, then enter a name and username (must end in "bot")',
      code: "/newbot",
    },
    {
      title: "Copy your token",
      desc: "BotFather will give you a token like 123456789:AAF... — copy it",
      tip: "Keep this token secret! Never share it publicly.",
    },
    {
      title: "Paste token below",
      desc: "Come back here and paste it in the form to register your bot.",
    },
  ];

  return (
    <div className="rounded-2xl border border-primary/20 bg-primary/[0.03] p-4 space-y-3">
      <div className="flex items-center gap-2">
        <div className="h-7 w-7 rounded-lg bg-primary/15 border border-primary/25 flex items-center justify-center">
          <Sparkles className="h-3.5 w-3.5 text-primary" />
        </div>
        <div>
          <p className="text-xs font-semibold text-foreground">How to create a Telegram Bot</p>
          <p className="text-[10px] text-muted-foreground/60">Via @BotFather — takes ~1 minute</p>
        </div>
      </div>

      <div className="space-y-2">
        {steps.map((s, i) => (
          <div key={i} className={cn(
            "flex gap-3 p-2.5 rounded-xl border transition-all",
            i === step ? "border-primary/30 bg-primary/5" : "border-border/20 opacity-60"
          )}>
            <div className={cn(
              "h-5 w-5 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0 mt-0.5",
              i < step ? "bg-emerald-400/20 text-emerald-400 border border-emerald-400/30" :
              i === step ? "bg-primary/20 text-primary border border-primary/30" :
              "bg-muted/30 text-muted-foreground border border-border/30"
            )}>
              {i < step ? "✓" : i + 1}
            </div>
            <div className="flex-1 min-w-0 space-y-1.5">
              <p className="text-xs font-medium text-foreground">{s.title}</p>
              <p className="text-[10px] text-muted-foreground/60">{s.desc}</p>
              {s.code && (
                <div className="flex items-center gap-2">
                  <code className="text-[11px] font-mono bg-[#0f1014] px-2 py-0.5 rounded border border-border/40 text-cyan-300">
                    {s.code}
                  </code>
                  <button
                    onClick={async () => {
                      if (!s.code) return;
                      await navigator.clipboard.writeText(s.code).catch(()=>{});
                      setCopied(true); setTimeout(()=>setCopied(false), 2000);
                    }}
                    className="text-[10px] text-muted-foreground hover:text-foreground transition-colors"
                  >
                    {copied ? <Check className="h-3 w-3 text-emerald-400"/> : <Copy className="h-3 w-3"/>}
                  </button>
                </div>
              )}
              {s.tip && (
                <p className="text-[10px] text-amber-400/70 flex items-center gap-1">
                  <Shield className="h-2.5 w-2.5" />{s.tip}
                </p>
              )}
              {s.action && <div>{s.action}</div>}
            </div>
          </div>
        ))}
      </div>

      <div className="flex gap-2 justify-end">
        {step > 0 && (
          <button onClick={() => setStep(s=>s-1)} className="px-3 py-1.5 text-xs rounded-lg border border-border/40 text-muted-foreground hover:text-foreground transition-colors">Back</button>
        )}
        {step < steps.length - 1 ? (
          <button onClick={() => setStep(s=>s+1)} className="px-3 py-1.5 text-xs rounded-lg bg-primary/10 text-primary border border-primary/25 hover:bg-primary/20 transition-all">
            Next step →
          </button>
        ) : (
          <button onClick={() => setStep(0)} className="px-3 py-1.5 text-xs rounded-lg bg-emerald-400/10 text-emerald-400 border border-emerald-400/25 hover:bg-emerald-400/20 transition-all">
            ✓ Got my token!
          </button>
        )}
      </div>
    </div>
  );
}

// ── Bot Card ───────────────────────────────────────────────────────────────────
function BotCard({ bot, onDeploy, onDelete, isDeploying }: {
  bot: any; onDeploy: (id: string) => void; onDelete: (id: string) => void; isDeploying: boolean;
}) {
  const status = STATUS_META[bot.status] || STATUS_META.pending;
  const StatusIcon = status.Icon;
  const [showDetails, setShowDetails] = useState(false);

  return (
    <div className={cn("rounded-2xl border bg-card/40 p-4 space-y-3 transition-all duration-200 animate-slide-up hover:border-border hover:bg-card/60", status.border)}>
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className={cn("h-10 w-10 rounded-xl border flex items-center justify-center shrink-0", status.bg, status.border)}>
            <Bot className={cn("h-5 w-5", status.color)} />
          </div>
          <div>
            <p className="text-sm font-semibold">{bot.name}</p>
            <p className="text-xs text-muted-foreground/70 font-mono">@{bot.username}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Badge className={cn("text-[10px] font-mono border shrink-0", status.color, status.bg, status.border)}>
            <StatusIcon className={cn("h-2.5 w-2.5 mr-1", bot.status === "deploying" && "animate-spin")} />
            {status.label}
          </Badge>
          <button onClick={() => setShowDetails(d=>!d)} className="p-1.5 rounded-lg text-muted-foreground/50 hover:text-foreground hover:bg-white/[0.05] transition-all">
            <Settings2 className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {bot.description && <p className="text-xs text-muted-foreground/60 leading-relaxed">{bot.description}</p>}

      {showDetails && (
        <div className="rounded-xl bg-card/30 border border-border/30 p-3 space-y-2 text-[10px] font-mono">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground/50">ID</span>
            <span className="text-foreground/60">{bot.id?.slice(0,16)}…</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground/50">Created</span>
            <span className="text-foreground/60">{bot.created_at ? new Date(bot.created_at).toLocaleDateString() : "—"}</span>
          </div>
        </div>
      )}

      {(bot.hfSpaceUrl || bot.hf_space_url) && (
        <div className="flex flex-col gap-1">
          <a href={bot.hfSpaceUrl || bot.hf_space_url} target="_blank" rel="noopener noreferrer"
            className="flex items-center gap-1.5 text-[10px] text-primary/70 hover:text-primary font-mono transition-colors">
            <Globe className="h-3 w-3" />
            {(bot.hfSpaceUrl || bot.hf_space_url).replace("https://", "")}
          </a>
          {/* Log URL: replace /bots/ with /logs/ to deep-link to prdcn logs */}
          {(bot.hfSpaceUrl || bot.hf_space_url)?.includes("prdcn") && (
            <a href={(bot.hfSpaceUrl || bot.hf_space_url).replace("/bots/", "/logs/")} target="_blank" rel="noopener noreferrer"
              className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60 hover:text-foreground font-mono transition-colors">
              <Terminal className="h-3 w-3" />View logs
            </a>
          )}
        </div>
      )}

      <div className="flex items-center gap-2 pt-1 border-t border-border/30">
        <button onClick={() => onDeploy(bot.id)} disabled={isDeploying || bot.status === "deploying"}
          className={cn("flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all",
            "bg-primary/10 text-primary border border-primary/25 hover:bg-primary/20",
            "disabled:opacity-40 disabled:cursor-not-allowed active:scale-95")}>
          {isDeploying ? <Loader2 className="h-3 w-3 animate-spin" /> : <Rocket className="h-3 w-3" />}
          {bot.status === "active" ? "Redeploy" : "Deploy"}
        </button>
        <a href={`https://t.me/${bot.username}`} target="_blank" rel="noopener noreferrer"
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-muted-foreground border border-border/40 hover:text-foreground hover:border-border transition-all">
          <MessageSquare className="h-3 w-3" />Open
        </a>
        <button onClick={() => onDelete(bot.id)}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-muted-foreground border border-border/40 hover:text-rose-400 hover:border-rose-400/30 hover:bg-rose-400/5 transition-all active:scale-95 ml-auto">
          <Trash2 className="h-3 w-3" />Del
        </button>
      </div>
    </div>
  );
}

// ── Main Page ──────────────────────────────────────────────────────────────────
export default function BotsPage() {
  const { data: bots = [], isLoading } = useBots();
  const { create, deploy, remove, isCreating } = useBotMutations();
  const { toast } = useToast();
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [deployingId, setDeployingId] = useState<string | null>(null);
  const [showGuide, setShowGuide] = useState(false);

  const form = useForm<BotFormData>({
    resolver: zodResolver(botSchema),
    defaultValues: { name: "", token: "", description: "", purpose: "" },
  });

  async function onSubmit(data: BotFormData) {
    // Auto-derive username from name if not explicitly provided
    const autoUsername = data.name.toLowerCase().replace(/[^a-z0-9]/g, "_") + "_bot";

    try {
      await create({
        name: data.name,
        username: autoUsername,
        description: data.description || data.purpose,
        bot_token: data.token,  // Pass token to backend for storage
      });
      toast({ title: "✅ Bot registered", description: `@${autoUsername} is ready to deploy.` });
      form.reset();
      setIsDialogOpen(false);
    } catch (e: any) {
      toast({ title: "Error", description: e.message || "Failed to create bot", variant: "destructive" });
    }
  }

  async function handleDeploy(id: string) {
    setDeployingId(id);
    try {
      const result: any = await deploy(id);
      const msg = result?.message || "Bot queued for deployment — will be live in ~5s";
      toast({ title: "🚀 Deploying", description: msg });
    } catch (e: any) {
      toast({ title: "Deploy failed", description: e.message || "Failed to deploy", variant: "destructive" });
    } finally { setDeployingId(null); }
  }

  async function handleDelete(id: string) {
    // Use toast-confirm pattern (confirm() is blocked in Telegram WebApp)
    try {
      await remove(id);
      toast({ title: "Bot deleted" });
    } catch (e: any) {
      toast({ title: "Delete failed", description: e.message || "Failed to delete", variant: "destructive" });
    }
  }

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 pt-4 pb-6 space-y-5 max-w-lg mx-auto w-full">

          {/* Header */}
          <div className="flex items-center justify-between animate-slide-up">
            <div>
              <h1 className="text-base font-semibold tracking-tight">My Bots</h1>
              <p className="text-xs text-muted-foreground/60 mt-0.5">Deploy Telegram bots built by your agent</p>
            </div>
            <button onClick={() => setIsDialogOpen(true)}
              className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-md shadow-primary/20">
              <Plus className="h-3.5 w-3.5" />New Bot
            </button>
          </div>

          {/* How-to guide toggle */}
          <button onClick={() => setShowGuide(g=>!g)}
            className={cn("w-full flex items-center justify-between px-3 py-2.5 rounded-xl border text-xs transition-all",
              showGuide ? "border-primary/30 bg-primary/5 text-primary" : "border-border/40 text-muted-foreground hover:text-foreground hover:border-border/70")}>
            <div className="flex items-center gap-2">
              <Key className="h-3.5 w-3.5" />
              <span>How to get a Bot Token from Telegram</span>
            </div>
            <ChevronRight className={cn("h-3.5 w-3.5 transition-transform", showGuide && "rotate-90")} />
          </button>

          {showGuide && <ManagedBotGuide />}

          {/* Bot list */}
          {isLoading ? (
            <div className="flex justify-center py-12"><Loader2 className="h-5 w-5 animate-spin text-primary/60" /></div>
          ) : bots.length === 0 ? (
            <div className="flex flex-col items-center py-12 gap-4 text-center animate-fade-in">
              <div className="h-14 w-14 rounded-2xl bg-primary/5 border border-primary/15 flex items-center justify-center animate-float">
                <Bot className="h-7 w-7 text-primary/40" />
              </div>
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">No bots yet</p>
                <p className="text-xs text-muted-foreground/50 max-w-56">Build a bot with your agent, then register and deploy it here.</p>
              </div>
              <button onClick={() => setIsDialogOpen(true)}
                className="flex items-center gap-2 px-4 py-2 rounded-xl border border-primary/30 text-primary text-xs hover:bg-primary/5 transition-all">
                <Plus className="h-3.5 w-3.5" />Register First Bot
              </button>
            </div>
          ) : (
            <div className="space-y-3 stagger">
              {bots.map(bot => (
                <BotCard key={bot.id} bot={bot} onDeploy={handleDeploy} onDelete={handleDelete} isDeploying={deployingId === bot.id} />
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Register bot dialog */}
      <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
        <DialogContent className="bg-card border-border/60 rounded-2xl max-w-sm mx-auto">
          <DialogHeader>
            <DialogTitle className="text-base flex items-center gap-2">
              <Bot className="h-4 w-4 text-primary" />Register Bot
            </DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              Connect a Telegram bot token and deploy it automatically.
            </DialogDescription>
          </DialogHeader>
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4 pt-1">
              <FormField control={form.control} name="name" render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-xs">Bot Name</FormLabel>
                  <FormControl><Input placeholder="My Rust Bot" {...field} className="text-sm" /></FormControl>
                  <FormMessage className="text-xs" />
                </FormItem>
              )} />
              <FormField control={form.control} name="token" render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-xs flex items-center gap-1.5">
                    <Key className="h-3 w-3 text-primary/60" />Bot Token <span className="text-rose-400">*</span>
                  </FormLabel>
                  <FormControl>
                    <Input placeholder="123456789:AAF…" {...field} className="text-sm font-mono" type="password" />
                  </FormControl>
                  <FormDescription className="text-[10px] flex items-center gap-1 text-muted-foreground/50">
                    <Shield className="h-2.5 w-2.5" />Get from @BotFather · stored encrypted
                  </FormDescription>
                  <FormMessage className="text-xs" />
                </FormItem>
              )} />
              <FormField control={form.control} name="purpose" render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-xs">What should this bot do? <span className="text-muted-foreground/40">(optional)</span></FormLabel>
                  <FormControl>
                    <Input placeholder="e.g. Answer customer questions about my store" {...field} className="text-sm" />
                  </FormControl>
                  <FormDescription className="text-[10px] text-muted-foreground/50">The agent will use this to configure your bot</FormDescription>
                </FormItem>
              )} />
              <FormField control={form.control} name="description" render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-xs">Short description <span className="text-muted-foreground/40">(optional)</span></FormLabel>
                  <FormControl>
                    <Input placeholder="Customer support bot for Acme Corp" {...field} className="text-sm" />
                  </FormControl>
                </FormItem>
              )} />
              <DialogFooter className="pt-2 gap-2">
                <Button type="button" variant="ghost" size="sm" onClick={() => setIsDialogOpen(false)}>Cancel</Button>
                <Button type="submit" size="sm" disabled={isCreating} className="gap-2">
                  {isCreating && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
                  Register Bot
                </Button>
              </DialogFooter>
            </form>
          </Form>
        </DialogContent>
      </Dialog>
    </AppLayout>
  );
}
