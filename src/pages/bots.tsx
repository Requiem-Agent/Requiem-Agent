import { useState } from "react";
import { AppLayout } from "@/components/layout";
import { useBots, useBotMutations, useBotStatusPoller, TRANSIENT_STATUSES } from "@/hooks/use-bots";
import { useToast } from "@/hooks/use-toast";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import * as z from "zod";
import {
  Plus, Bot, Rocket, Trash2, Loader2,
  CheckCircle2, AlertCircle, Clock, Zap, RefreshCw,
  MessageSquare, Settings2, Globe, Key, Copy, Check,
  ChevronRight, Terminal, ArrowRight, Sparkles, Shield,
  Link2, ChevronDown,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Form, FormField, FormItem, FormLabel, FormControl, FormDescription, FormMessage } from "@/components/ui/form";
import { cn } from "@/lib/utils";

// ── Schemas ────────────────────────────────────────────────────────────────────
const provisionSchema = z.object({
  name:        z.string().min(2, "حرفين على الأقل"),
  description: z.string().optional(),
  purpose:     z.string().optional(),
});
type ProvisionFormData = z.infer<typeof provisionSchema>;

const linkTokenSchema = z.object({
  token: z.string().min(10, "الصق توكن البوت من @BotFather"),
});
type LinkTokenFormData = z.infer<typeof linkTokenSchema>;

// Legacy schema kept for type compat
const botSchema = z.object({
  name:        z.string().min(2, "حرفين على الأقل"),
  token:       z.string().min(10, "الصق توكن البوت من @BotFather"),
  description: z.string().optional(),
  purpose:     z.string().optional(),
});
type BotFormData = z.infer<typeof botSchema>;

// ── Status map ─────────────────────────────────────────────────────────────────
const STATUS_META: Record<string, { label: string; color: string; bg: string; border: string; Icon: React.ElementType }> = {
  active:    { label: "نشط",    color: "text-neutral-700", bg: "bg-neutral-100", border: "border-neutral-300", Icon: CheckCircle2 },
  deploying: { label: "جاري النشر", color: "text-neutral-500", bg: "bg-neutral-100", border: "border-neutral-300", Icon: RefreshCw    },
  building:  { label: "جاري البناء",  color: "text-neutral-600", bg: "bg-neutral-100", border: "border-neutral-300", Icon: Zap          },
  deployed:  { label: "تم النشر",  color: "text-neutral-600", bg: "bg-neutral-100", border: "border-neutral-300", Icon: CheckCircle2 },
  error:     { label: "خطأ",     color: "text-red-600",     bg: "bg-red-50",      border: "border-red-200",      Icon: AlertCircle  },
  pending:   { label: "معلّق",   color: "text-muted-foreground", bg: "bg-neutral-100",  border: "border-border",         Icon: Clock        },
  sleeping:  { label: "خامل",  color: "text-muted-foreground", bg: "bg-neutral-50",  border: "border-border/50",      Icon: Clock        },
};

// ── BotFather step guide (static, shown on guide toggle) ─────────────────────
function BotFatherGuide() {
  const [step, setStep] = useState(0);
  const [copied, setCopied] = useState(false);

  const steps = [
    {
      title: "فتح @BotFather",
      desc: "ابدأ محادثة مع @BotFather في تيليجرام",
      action: (
        <a href="https://t.me/BotFather" target="_blank" rel="noopener noreferrer"
          className="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-neutral-100 text-black border border-neutral-300 text-xs font-medium hover:bg-neutral-200 transition-all">
          <ArrowRight className="h-3.5 w-3.5" />فتح @BotFather
        </a>
      )
    },
    {
      title: "إنشاء بوت جديد",
      desc: 'أرسل /newbot، ثم أدخل اسماً واسم مستخدم (يجب أن ينتهي بـ "bot")',
      code: "/newbot",
    },
    {
      title: "انسخ التوكن",
      desc: "سيعطيك BotFather توكن مثل 123456789:AAF... — انسخه",
      tip: "احتفظ بهذا التوكن سرياً! لا تشاركه أبداً.",
    },
    {
      title: "الصق التوكن أدناه",
      desc: "عد هنا والصقه في النموذج لتسجيل بوتوك.",
    },
  ];

  return (
    <div className="rounded-2xl border border-neutral-200 bg-neutral-50 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <div className="h-7 w-7 rounded-lg bg-neutral-100 border border-neutral-300 flex items-center justify-center">
          <Sparkles className="h-3.5 w-3.5 text-black" />
        </div>
        <div>
          <p className="text-xs font-semibold text-foreground">كيفية إنشاء بوت تيليجرام</p>
          <p className="text-[10px] text-muted-foreground/60">عبر @BotFather — يستغرق حوالي دقيقة</p>
        </div>
      </div>

      <div className="space-y-2">
        {steps.map((s, i) => (
          <div key={i} className={cn(
            "flex gap-3 p-2.5 rounded-xl border transition-all",
            i === step ? "border-neutral-200 bg-neutral-50" : "border-border/20 opacity-60"
          )}>
            <div className={cn(
              "h-5 w-5 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0 mt-0.5",
              i < step ? "bg-neutral-100 text-neutral-700 border border-neutral-300" :
              i === step ? "bg-neutral-100 text-black border border-neutral-200" :
              "bg-muted/30 text-muted-foreground border border-border/30"
            )}>
              {i < step ? "✓" : i + 1}
            </div>
            <div className="flex-1 min-w-0 space-y-1.5">
              <p className="text-xs font-medium text-foreground">{s.title}</p>
              <p className="text-[10px] text-muted-foreground/60">{s.desc}</p>
              {s.code && (
                <div className="flex items-center gap-2">
                    <code className="text-[11px] font-mono bg-[#0f1014] px-2 py-0.5 rounded border border-border/40 text-neutral-600">
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
                    {copied ? <Check className="h-3 w-3 text-neutral-700"/> : <Copy className="h-3 w-3"/>}
                  </button>
                </div>
              )}
              {s.tip && (
                <p className="text-[10px] text-neutral-500 flex items-center gap-1">
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
          <button onClick={() => setStep(s=>s-1)} className="px-3 py-1.5 text-xs rounded-lg border border-border/40 text-muted-foreground hover:text-foreground transition-colors">رجوع</button>
        )}
        {step < steps.length - 1 ? (
          <button onClick={() => setStep(s=>s+1)} className="px-3 py-1.5 text-xs rounded-lg bg-neutral-100 text-black border border-neutral-300 hover:bg-neutral-200 transition-all">
            الخطوة التالية →
          </button>
        ) : (
          <button onClick={() => setStep(0)} className="px-3 py-1.5 text-xs rounded-lg bg-neutral-100 text-neutral-700 border border-neutral-300 hover:bg-neutral-200 transition-all">
            ✓ حصلت على التوكن!
          </button>
        )}
      </div>
    </div>
  );
}

// ── Provisioned bot info panel (shows suggested name/username + steps) ────────
function ProvisionedSteps({ suggestedName, suggestedUsername, botfatherSteps }: {
  suggestedName: string;
  suggestedUsername: string;
  botfatherSteps?: string[];
}) {
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const copyText = async (text: string, field: string) => {
    await navigator.clipboard.writeText(text).catch(() => {});
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  const steps = botfatherSteps ?? [
    "1. افتح @BotFather في تيليجرام",
    `/newbot`,
    `أدخل الاسم: ${suggestedName}`,
    `أدخل اسم المستخدم: ${suggestedUsername}`,
    "انسخ التوكن الذي يعطيك BotFather",
  ];

  return (
    <div className="rounded-xl border border-neutral-300 bg-neutral-100 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <CheckCircle2 className="h-4 w-4 text-neutral-700 shrink-0" />
        <p className="text-xs font-semibold text-neutral-700">تم حجز مقعد البوت — أنشئه في تيليجرام</p>
      </div>

      <div className="space-y-2">
        {[
          { label: "اسم مقترح", value: suggestedName, field: "name" },
          { label: "اسم مستخدم مقترح", value: `@${suggestedUsername}`, field: "username" },
        ].map(({ label, value, field }) => (
          <div key={field} className="flex items-center justify-between gap-2 px-3 py-2 rounded-lg bg-card/40 border border-border/30">
            <div>
              <p className="text-[10px] text-muted-foreground/50">{label}</p>
              <p className="text-xs font-mono font-medium text-foreground">{value}</p>
            </div>
            <button onClick={() => copyText(value, field)}
              className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground transition-colors hover:bg-white/[0.05]">
              {copiedField === field ? <Check className="h-3 w-3 text-neutral-700" /> : <Copy className="h-3 w-3" />}
            </button>
          </div>
        ))}
      </div>

      <div className="space-y-1.5">
        <p className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider">خطوات BotFather:</p>
        {steps.map((step, i) => (
          <div key={i} className="flex items-start gap-2">
            <span className="h-4 w-4 rounded-full bg-neutral-100 text-black text-[9px] flex items-center justify-center font-bold shrink-0 mt-0.5">{i + 1}</span>
            <p className="text-[11px] text-muted-foreground/80 font-mono">{step}</p>
          </div>
        ))}
      </div>

      <a href="https://t.me/BotFather" target="_blank" rel="noopener noreferrer"
        className="flex items-center justify-center gap-2 w-full px-3 py-2 rounded-lg bg-neutral-100 text-black border border-neutral-300 text-xs font-medium hover:bg-neutral-200 transition-all">
        <ArrowRight className="h-3.5 w-3.5" />فتح @BotFather الآن
      </a>
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

  // Poll status while bot is in a transient/building state
  useBotStatusPoller(bot.id, TRANSIENT_STATUSES.has(bot.status));

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
            <span className="text-muted-foreground/50">المعرّف</span>
            <span className="text-foreground/60">{bot.id?.slice(0,16)}…</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground/50">تاريخ الإنشاء</span>
            <span className="text-foreground/60">{bot.created_at ? new Date(bot.created_at).toLocaleDateString() : "—"}</span>
          </div>
        </div>
      )}

      {(bot.hfSpaceUrl || bot.hf_space_url) && (
        <div className="flex flex-col gap-1">
          <a href={bot.hfSpaceUrl || bot.hf_space_url} target="_blank" rel="noopener noreferrer"
            className="flex items-center gap-1.5 text-[10px] text-neutral-600 hover:text-black font-mono transition-colors">
            <Globe className="h-3 w-3" />
            {(bot.hfSpaceUrl || bot.hf_space_url).replace("https://", "")}
          </a>
          {/* Log URL: replace /bots/ with /logs/ to deep-link to prdcn logs */}
          {(bot.hfSpaceUrl || bot.hf_space_url)?.includes("prdcn") && (
            <a href={(bot.hfSpaceUrl || bot.hf_space_url).replace("/bots/", "/logs/")} target="_blank" rel="noopener noreferrer"
              className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60 hover:text-foreground font-mono transition-colors">
               <Terminal className="h-3 w-3" />عرض السجلات
            </a>
          )}
        </div>
      )}

      <div className="flex items-center gap-2 pt-1 border-t border-border/30">
        <button onClick={() => onDeploy(bot.id)} disabled={isDeploying || bot.status === "deploying"}
          className={cn("flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all",
            "bg-neutral-100 text-black border border-neutral-300 hover:bg-neutral-200",
            "disabled:opacity-40 disabled:cursor-not-allowed active:scale-95")}>
          {isDeploying ? <Loader2 className="h-3 w-3 animate-spin" /> : <Rocket className="h-3 w-3" />}
          {bot.status === "active" ? "إعادة نشر" : "نشر"}
        </button>
        <a href={`https://t.me/${bot.username}`} target="_blank" rel="noopener noreferrer"
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-muted-foreground border border-border/40 hover:text-foreground hover:border-border transition-all">
           <MessageSquare className="h-3 w-3" />فتح
        </a>
        <button onClick={() => onDelete(bot.id)}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-muted-foreground border border-border/40 hover:text-red-600 hover:border-red-200 hover:bg-red-50 transition-all active:scale-95 ml-auto">
          <Trash2 className="h-3 w-3" />حذف
        </button>
      </div>
    </div>
  );
}

// ── 2-step New Bot Dialog ─────────────────────────────────────────────────────
type ProvisionResult = {
  bot_id: string;
  suggested_name: string;
  suggested_username: string;
  botfather_steps?: string[];
  message?: string;
};

function NewBotDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { provision, linkToken, deploy } = useBotMutations();
  const { toast } = useToast();

  // Step: 1 = provision form, 2 = link token
  const [dialogStep, setDialogStep] = useState<1 | 2>(1);
  const [provisioned, setProvisioned] = useState<ProvisionResult | null>(null);

  const provisionForm = useForm<ProvisionFormData>({
    resolver: zodResolver(provisionSchema),
    defaultValues: { name: "", description: "", purpose: "" },
  });

  const linkForm = useForm<LinkTokenFormData>({
    resolver: zodResolver(linkTokenSchema),
    defaultValues: { token: "" },
  });

  function handleClose() {
    setDialogStep(1);
    setProvisioned(null);
    provisionForm.reset();
    linkForm.reset();
    onClose();
  }

  async function onProvision(data: ProvisionFormData) {
    try {
      const result: ProvisionResult = await provision.mutateAsync(data);
      setProvisioned(result);
      setDialogStep(2);
      toast({ title: "✅ تم حجز مقعد البوت", description: result.message || "اتبع خطوات BotFather لإنشائه في تيليجرام." });
    } catch (e: any) {
      toast({ title: "فشل الحجز", description: e.message || "تعذر حجز البوت", variant: "destructive" });
    }
  }

  async function onLinkToken(data: LinkTokenFormData) {
    if (!provisioned) return;
    try {
      await linkToken.mutateAsync({ id: provisioned.bot_id, token: data.token });
      // Immediately trigger deploy after linking
      try {
        await deploy(provisioned.bot_id);
        toast({ title: "🚀 تم الربط والنشر", description: `@${provisioned.suggested_username} جاري نشره.` });
      } catch {
        toast({ title: "✅ تم ربط التوكن", description: "انشره يدوياً من قائمة البوتات." });
      }
      handleClose();
    } catch (e: any) {
      toast({ title: "فشل الربط", description: e.message || "تعذر ربط التوكن", variant: "destructive" });
    }
  }

  return (
    <Dialog open={open} onOpenChange={v => !v && handleClose()}>
      <DialogContent className="bg-card border-border/60 rounded-2xl max-w-sm mx-auto">
        {/* Step indicator */}
        <div className="flex items-center gap-2 mb-1">
          {[1, 2].map(s => (
            <div key={s} className={cn(
              "flex items-center gap-1.5 text-[10px] font-medium transition-all",
              s === dialogStep ? "text-black" : s < dialogStep ? "text-neutral-700" : "text-muted-foreground/40"
            )}>
              <div className={cn(
                "h-4 w-4 rounded-full flex items-center justify-center text-[9px] font-bold border",
                s === dialogStep ? "bg-neutral-100 border-neutral-200 text-black" :
                s < dialogStep ? "bg-neutral-100 border-neutral-300 text-neutral-700" :
                "bg-muted/20 border-border/30"
              )}>
                {s < dialogStep ? "✓" : s}
              </div>
              {s === 1 ? "حجز" : "ربط التوكن"}
              {s < 2 && <ChevronRight className="h-2.5 w-2.5 text-muted-foreground/30" />}
            </div>
          ))}
        </div>

        {dialogStep === 1 && (
          <>
            <DialogHeader>
              <DialogTitle className="text-base flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-black" />بوت جديد مُدار
              </DialogTitle>
              <DialogDescription className="text-xs text-muted-foreground">
                أخبر الوكيل ماذا يجب أن يفعل بوتك — سيقترح اسماً ويرشدك عبر BotFather.
              </DialogDescription>
            </DialogHeader>
            <Form {...provisionForm}>
              <form onSubmit={provisionForm.handleSubmit(onProvision)} className="space-y-4 pt-1">
                <FormField control={provisionForm.control} name="name" render={({ field }) => (
                  <FormItem>
                    <FormLabel className="text-xs">اسم البوت</FormLabel>
                    <FormControl><Input placeholder="بوت الدعم" {...field} className="text-sm" /></FormControl>
                    <FormMessage className="text-xs" />
                  </FormItem>
                )} />
                <FormField control={provisionForm.control} name="purpose" render={({ field }) => (
                  <FormItem>
                    <FormLabel className="text-xs">الغرض <span className="text-muted-foreground/40">(اختياري)</span></FormLabel>
                    <FormControl>
                      <Input placeholder="مثال: الإجابة على أسئلة العملاء عن متجرنا" {...field} className="text-sm" />
                    </FormControl>
                    <FormDescription className="text-[10px] text-muted-foreground/50">يستخدم الوكيل هذا لتكوين البوت</FormDescription>
                  </FormItem>
                )} />
                <FormField control={provisionForm.control} name="description" render={({ field }) => (
                  <FormItem>
                    <FormLabel className="text-xs">وصف قصير <span className="text-muted-foreground/40">(اختياري)</span></FormLabel>
                    <FormControl>
                      <Input placeholder="بوت دعم فني لشركة Acme" {...field} className="text-sm" />
                    </FormControl>
                  </FormItem>
                )} />
                <DialogFooter className="pt-2 gap-2">
                  <Button type="button" variant="ghost" size="sm" onClick={handleClose}>إلغاء</Button>
                  <Button type="submit" size="sm" disabled={provision.isPending} className="gap-2">
                    {provision.isPending && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
                    <Rocket className="h-3.5 w-3.5" />حجز البوت
                  </Button>
                </DialogFooter>
              </form>
            </Form>
          </>
        )}

        {dialogStep === 2 && provisioned && (
          <>
            <DialogHeader>
              <DialogTitle className="text-base flex items-center gap-2">
                <Link2 className="h-4 w-4 text-black" />ربط توكن BotFather
              </DialogTitle>
              <DialogDescription className="text-xs text-muted-foreground">
                أنشئ البوت في تيليجرام باستخدام التفاصيل أدناه، ثم الصق التوكن هنا.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 pt-1">
              <ProvisionedSteps
                suggestedName={provisioned.suggested_name}
                suggestedUsername={provisioned.suggested_username}
                botfatherSteps={provisioned.botfather_steps}
              />
              <Form {...linkForm}>
                <form onSubmit={linkForm.handleSubmit(onLinkToken)} className="space-y-4">
                  <FormField control={linkForm.control} name="token" render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-xs flex items-center gap-1.5">
                        <Key className="h-3 w-3 text-neutral-600" />الصق توكن BotFather
                      </FormLabel>
                      <FormControl>
                        <Input placeholder="123456789:AAF…" {...field} className="text-sm font-mono" type="password" />
                      </FormControl>
                      <FormDescription className="text-[10px] flex items-center gap-1 text-muted-foreground/50">
                        <Shield className="h-2.5 w-2.5" />مخزن بشكل مشفر · لا يُشارك أبداً
                      </FormDescription>
                      <FormMessage className="text-xs" />
                    </FormItem>
                  )} />
                  <DialogFooter className="gap-2">
                    <Button type="button" variant="ghost" size="sm" onClick={() => setDialogStep(1)}>← رجوع</Button>
                    <Button type="submit" size="sm" disabled={linkToken.isPending} className="gap-2">
                      {linkToken.isPending && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
                      <Rocket className="h-3.5 w-3.5" />ربط ونشر
                    </Button>
                  </DialogFooter>
                </form>
              </Form>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

// ── Main Page ──────────────────────────────────────────────────────────────────
export default function BotsPage() {
  const { data: bots = [], isLoading } = useBots();
  const { deploy, remove } = useBotMutations();
  const { toast } = useToast();
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [deployingId, setDeployingId] = useState<string | null>(null);
  const [showGuide, setShowGuide] = useState(false);

  async function handleDeploy(id: string) {
    setDeployingId(id);
    try {
      const result: any = await deploy(id);
      const msg = result?.message || "تمت إضافة البوت للنشر — سيكون نشطاً خلال ~5 ثوانٍ";
      toast({ title: "🚀 Deploying", description: msg });
    } catch (e: any) {
      toast({ title: "فشل النشر", description: e.message || "فشل في النشر", variant: "destructive" });
    } finally { setDeployingId(null); }
  }

  async function handleDelete(id: string) {
    try {
      await remove(id);
      toast({ title: "تم حذف البوت" });
    } catch (e: any) {
      toast({ title: "فشل الحذف", description: e.message || "فشل في الحذف", variant: "destructive" });
    }
  }

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="px-4 pt-4 pb-6 space-y-5 max-w-lg mx-auto w-full">

          {/* Header */}
          <div className="flex items-center justify-between animate-slide-up">
            <div>
              <h1 className="text-base font-semibold tracking-tight">بوتاتي</h1>
              <p className="text-xs text-muted-foreground/60 mt-0.5">انشر بوتات تيليجرام المبنية بواسطة الوكيل</p>
            </div>
            <button onClick={() => setIsDialogOpen(true)}
              className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-black text-white text-xs font-medium hover:bg-neutral-800 transition-all active:scale-95 shadow-md shadow-black/20">
               <Plus className="h-3.5 w-3.5" />بوت جديد
            </button>
          </div>

          {/* How-to guide toggle */}
          <button onClick={() => setShowGuide(g=>!g)}
            className={cn("w-full flex items-center justify-between px-3 py-2.5 rounded-xl border text-xs transition-all",
              showGuide ? "border-neutral-200 bg-neutral-50 text-black" : "border-border/40 text-muted-foreground hover:text-foreground hover:border-border/70")}>
            <div className="flex items-center gap-2">
              <Key className="h-3.5 w-3.5" />
              <span>كيفية الحصول على توكن بوت من تيليجرام</span>
            </div>
            <ChevronDown className={cn("h-3.5 w-3.5 transition-transform", showGuide && "rotate-180")} />
          </button>

          {showGuide && <BotFatherGuide />}

          {/* Bot list */}
          {isLoading ? (
            <div className="flex justify-center py-12"><Loader2 className="h-5 w-5 animate-spin text-neutral-600" /></div>
          ) : bots.length === 0 ? (
            <div className="flex flex-col items-center py-12 gap-4 text-center animate-fade-in">
              <div className="h-14 w-14 rounded-2xl bg-neutral-50 border border-neutral-200 flex items-center justify-center animate-float">
                <Bot className="h-7 w-7 text-neutral-600" />
              </div>
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">لا توجد بوتات بعد</p>
                <p className="text-xs text-muted-foreground/50 max-w-56">ابنِ بوتاً مع الوكيل، ثم سجّله وانشره هنا.</p>
              </div>
              <button onClick={() => setIsDialogOpen(true)}
                className="flex items-center gap-2 px-4 py-2 rounded-xl border border-neutral-200 text-black text-xs hover:bg-neutral-50 transition-all">
                <Plus className="h-3.5 w-3.5" />إنشاء أول بوت
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

      <NewBotDialog open={isDialogOpen} onClose={() => setIsDialogOpen(false)} />
    </AppLayout>
  );
}
