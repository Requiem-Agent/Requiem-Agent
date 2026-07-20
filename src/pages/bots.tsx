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
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Form, FormField, FormItem, FormLabel, FormControl, FormDescription, FormMessage } from "@/components/ui/form";
import { cn } from "@/lib/utils";

const botSchema = z.object({
  name:        z.string().min(2, "Name must be at least 2 characters"),
  username:    z.string().min(3, "Username must be at least 3 characters"),
  description: z.string().optional(),
});
type BotFormData = z.infer<typeof botSchema>;

const STATUS_META: Record<string, { label: string; color: string; bg: string; border: string; Icon: React.ElementType }> = {
  active:    { label: "active",    color: "text-emerald-400", bg: "bg-emerald-400/10", border: "border-emerald-400/25", Icon: CheckCircle2 },
  deploying: { label: "deploying", color: "text-amber-400",   bg: "bg-amber-400/10",   border: "border-amber-400/25",   Icon: RefreshCw    },
  building:  { label: "building",  color: "text-cyan-400",    bg: "bg-cyan-400/10",    border: "border-cyan-400/25",    Icon: Zap          },
  deployed:  { label: "deployed",  color: "text-cyan-400",    bg: "bg-cyan-400/10",    border: "border-cyan-400/25",    Icon: CheckCircle2 },
  error:     { label: "error",     color: "text-rose-400",    bg: "bg-rose-400/10",    border: "border-rose-400/25",    Icon: AlertCircle  },
  pending:   { label: "pending",   color: "text-muted-foreground", bg: "bg-muted/40",  border: "border-border",         Icon: Clock        },
  sleeping:  { label: "sleeping",  color: "text-muted-foreground", bg: "bg-muted/20",  border: "border-border/50",      Icon: Clock        },
};

function BotCard({
  bot, onDeploy, onDelete, isDeploying,
}: {
  bot: any; onDeploy: (id: string) => void; onDelete: (id: string) => void; isDeploying: boolean;
}) {
  const status = STATUS_META[bot.status] || STATUS_META.pending;
  const StatusIcon = status.Icon;

  return (
    <div className={cn(
      "rounded-2xl border bg-card/40 p-4 space-y-3 transition-all duration-200 animate-slide-up",
      "hover:border-border hover:bg-card/60",
      status.border
    )}>
      {/* Header */}
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
        <Badge className={cn("text-[10px] font-mono border shrink-0", status.color, status.bg, status.border)}>
          <StatusIcon className={cn("h-2.5 w-2.5 mr-1", bot.status === "deploying" && "animate-spin")} />
          {status.label}
        </Badge>
      </div>

      {/* Description */}
      {bot.description && (
        <p className="text-xs text-muted-foreground/60 leading-relaxed">{bot.description}</p>
      )}

      {/* HF URL */}
      {bot.hf_space_url && (
        <a
          href={bot.hf_space_url}
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-1.5 text-[10px] text-primary/70 hover:text-primary font-mono transition-colors"
        >
          <ExternalLink className="h-3 w-3" />
          {bot.hf_space_url.replace("https://", "")}
        </a>
      )}

      {/* Actions */}
      <div className="flex items-center gap-2 pt-1 border-t border-border/30">
        <button
          onClick={() => onDeploy(bot.id)}
          disabled={isDeploying || bot.status === "deploying"}
          className={cn(
            "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all",
            "bg-primary/10 text-primary border border-primary/25 hover:bg-primary/20",
            "disabled:opacity-40 disabled:cursor-not-allowed active:scale-95"
          )}
        >
          {isDeploying
            ? <Loader2 className="h-3 w-3 animate-spin" />
            : <Rocket className="h-3 w-3" />
          }
          {bot.status === "active" ? "Redeploy" : "Deploy"}
        </button>

        <button
          onClick={() => onDelete(bot.id)}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-muted-foreground border border-border/40 hover:text-rose-400 hover:border-rose-400/30 hover:bg-rose-400/5 transition-all active:scale-95"
        >
          <Trash2 className="h-3 w-3" />
          Delete
        </button>
      </div>
    </div>
  );
}

export default function BotsPage() {
  const { data: bots = [], isLoading } = useBots();
  const { create, deploy, remove, isCreating } = useBotMutations();
  const { toast } = useToast();
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [deployingId, setDeployingId] = useState<string | null>(null);

  const form = useForm<BotFormData>({
    resolver: zodResolver(botSchema),
    defaultValues: { name: "", username: "", description: "" },
  });

  async function onSubmit(data: BotFormData) {
    try {
      await create({
        name: data.name,
        username: data.username.replace(/^@/, ""),
        description: data.description,
      });
      toast({ title: "Bot created", description: "Your bot has been provisioned." });
      form.reset();
      setIsDialogOpen(false);
    } catch (e: any) {
      toast({ title: "Error", description: e.message || "Failed to create bot", variant: "destructive" });
    }
  }

  async function handleDeploy(id: string) {
    setDeployingId(id);
    try {
      await deploy(id);
      toast({ title: "Deploy triggered", description: "Bot is deploying to HF Spaces." });
    } catch (e: any) {
      toast({ title: "Deploy failed", description: e.message || "Failed to deploy", variant: "destructive" });
    } finally {
      setDeployingId(null);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm("Delete this bot? This cannot be undone.")) return;
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
              <h1 className="text-base font-semibold tracking-tight">Bots</h1>
              <p className="text-xs text-muted-foreground/60 mt-0.5">Manage Telegram bots on HF Spaces</p>
            </div>
            <button
              onClick={() => setIsDialogOpen(true)}
              className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition-all active:scale-95 shadow-md shadow-primary/20"
            >
              <Plus className="h-3.5 w-3.5" />
              New Bot
            </button>
          </div>

          {/* Bot list */}
          {isLoading ? (
            <div className="flex justify-center py-12">
              <Loader2 className="h-5 w-5 animate-spin text-primary/60" />
            </div>
          ) : bots.length === 0 ? (
            <div className="flex flex-col items-center py-16 gap-4 text-center animate-fade-in">
              <div className="h-14 w-14 rounded-2xl bg-primary/5 border border-primary/15 flex items-center justify-center animate-float">
                <Bot className="h-7 w-7 text-primary/40" />
              </div>
              <div className="space-y-1">
                <p className="text-sm font-medium text-muted-foreground">No bots yet</p>
                <p className="text-xs text-muted-foreground/50 max-w-56">
                  Create your first Telegram bot and deploy it to Hugging Face Spaces.
                </p>
              </div>
              <button
                onClick={() => setIsDialogOpen(true)}
                className="flex items-center gap-2 px-4 py-2 rounded-xl border border-primary/30 text-primary text-xs hover:bg-primary/5 transition-all"
              >
                <Plus className="h-3.5 w-3.5" />
                Create your first bot
              </button>
            </div>
          ) : (
            <div className="space-y-3 stagger">
              {bots.map(bot => (
                <BotCard
                  key={bot.id}
                  bot={bot}
                  onDeploy={handleDeploy}
                  onDelete={handleDelete}
                  isDeploying={deployingId === bot.id}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Create dialog */}
      <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
        <DialogContent className="bg-card border-border/60 rounded-2xl">
          <DialogHeader>
            <DialogTitle className="text-base">Provision New Bot</DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              Enter bot details. Token generation and hosting are handled automatically.
            </DialogDescription>
          </DialogHeader>
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4 pt-2">
              <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel className="text-xs">Bot Name</FormLabel>
                    <FormControl>
                      <Input placeholder="My Helper Bot" {...field} className="text-sm" />
                    </FormControl>
                    <FormMessage className="text-xs" />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="username"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel className="text-xs">Bot Username</FormLabel>
                    <FormControl>
                      <Input placeholder="my_helper_bot" {...field} className="text-sm font-mono" />
                    </FormControl>
                    <FormDescription className="text-[10px]">Must end in 'bot'</FormDescription>
                    <FormMessage className="text-xs" />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="description"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel className="text-xs">Description <span className="text-muted-foreground/40">(optional)</span></FormLabel>
                    <FormControl>
                      <Input placeholder="What does this bot do?" {...field} className="text-sm" />
                    </FormControl>
                  </FormItem>
                )}
              />
              <DialogFooter className="pt-2">
                <Button type="button" variant="ghost" size="sm" onClick={() => setIsDialogOpen(false)}>Cancel</Button>
                <Button type="submit" size="sm" disabled={isCreating} className="gap-2">
                  {isCreating && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
                  Provision Bot
                </Button>
              </DialogFooter>
            </form>
          </Form>
        </DialogContent>
      </Dialog>
    </AppLayout>
  );
}
