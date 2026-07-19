import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import * as z from "zod";
import { Plus, Bot as BotIcon, Activity, Server, Trash2, Rocket, Loader2 } from "lucide-react";
import { AppLayout } from "@/components/layout";
import { useBots, useBotMutations } from "@/hooks/use-bots";
import { useToast } from "@/hooks/use-toast";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";

const botSchema = z.object({
  name: z.string().min(2, "Name must be at least 2 characters"),
  username: z.string().min(5, "Username must be at least 5 characters").regex(/bot$/i, "Username must end in 'bot'"),
});

export default function BotsPage() {
  const { data: bots = [], isLoading } = useBots();
  const { create, remove, deploy, isCreating, isDeploying } = useBotMutations();
  const { toast } = useToast();
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [deployingId, setDeployingId] = useState<string | null>(null);

  const form = useForm<z.infer<typeof botSchema>>({
    resolver: zodResolver(botSchema),
    defaultValues: {
      name: "",
      username: "",
    },
  });

  async function onSubmit(values: z.infer<typeof botSchema>) {
    try {
      await create(values);
      setIsDialogOpen(false);
      form.reset();
      toast({ title: "Bot created", description: "Your new bot has been provisioned." });
    } catch (e: any) {
      toast({ title: "Error", description: e.message || "Failed to create bot", variant: "destructive" });
    }
  }

  async function handleDeploy(id: string) {
    setDeployingId(id);
    try {
      await deploy(id);
      toast({ title: "Deploy triggered", description: "Your bot is deploying to HF Spaces." });
    } catch (e: any) {
      toast({ title: "Deploy failed", description: e.message || "Failed to deploy bot", variant: "destructive" });
    } finally {
      setDeployingId(null);
    }
  }

  async function handleDelete(id: string) {
    if (confirm("Are you sure you want to delete this bot? This cannot be undone.")) {
      try {
        await remove(id);
        toast({ title: "Bot deleted" });
      } catch (e: any) {
        toast({ title: "Delete failed", description: e.message || "Failed to delete bot", variant: "destructive" });
      }
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20';
      case 'building': return 'bg-amber-500/10 text-amber-500 border-amber-500/20';
      case 'deployed': return 'bg-cyan-500/10 text-cyan-500 border-cyan-500/20';
      case 'sleeping': return 'bg-muted text-muted-foreground border-border';
      case 'error': return 'bg-destructive/10 text-destructive border-destructive/20';
      default: return 'bg-primary/10 text-primary border-primary/20';
    }
  };

  return (
    <AppLayout>
      <div className="flex flex-col h-full overflow-y-auto p-4 md:p-8 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight text-foreground font-mono">/bots</h1>
            <p className="text-sm text-muted-foreground mt-1">Manage and deploy your managed Telegram bots.</p>
          </div>
          
          <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
            <DialogTrigger asChild>
              <Button size="sm" className="gap-2">
                <Plus className="h-4 w-4" />
                <span className="hidden sm:inline">Create Bot</span>
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Provision New Bot</DialogTitle>
                <DialogDescription>
                  Enter the details for your new Telegram bot. We'll handle the token generation and hosting.
                </DialogDescription>
              </DialogHeader>
              <Form {...form}>
                <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4 pt-4">
                  <FormField
                    control={form.control}
                    name="name"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Bot Name</FormLabel>
                        <FormControl>
                          <Input placeholder="e.g. My Helper Bot" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="username"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Bot Username</FormLabel>
                        <FormControl>
                          <Input placeholder="e.g. my_helper_bot" {...field} />
                        </FormControl>
                        <FormDescription>Must end in 'bot' and be unique globally.</FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <DialogFooter className="pt-4">
                    <Button type="submit" disabled={isCreating}>
                      {isCreating ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                      Provision Bot
                    </Button>
                  </DialogFooter>
                </form>
              </Form>
            </DialogContent>
          </Dialog>
        </div>

        {isLoading ? (
          <div className="flex items-center justify-center h-48">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : bots.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 border border-dashed border-border rounded-lg bg-card/30">
            <BotIcon className="h-12 w-12 text-muted-foreground mb-4 opacity-50" />
            <h3 className="text-lg font-medium text-foreground">No bots yet</h3>
            <p className="text-sm text-muted-foreground max-w-sm text-center mt-2 mb-6">
              Create your first bot to get started. We'll automatically provision it and prepare it for deployment.
            </p>
            <Button variant="outline" onClick={() => setIsDialogOpen(true)}>Create First Bot</Button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {bots.map((bot) => (
              <Card key={bot.id} className="flex flex-col border-border/50 bg-[#0d0d0f] hover:border-border transition-colors">
                <CardHeader className="pb-3">
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-3">
                      <div className="h-10 w-10 rounded-md bg-[#141416] border border-border flex items-center justify-center">
                        <BotIcon className="h-5 w-5 text-primary" />
                      </div>
                      <div>
                        <CardTitle className="text-base">{bot.name}</CardTitle>
                        <CardDescription className="font-mono text-xs mt-1">@{bot.username}</CardDescription>
                      </div>
                    </div>
                  </div>
                </CardHeader>
                <CardContent className="pb-4 flex-1">
                  <div className="flex items-center gap-2 mb-4">
                    <Badge variant="outline" className={`capitalize font-mono text-[10px] ${getStatusColor(bot.status)}`}>
                      <Activity className="h-3 w-3 mr-1" />
                      {bot.status}
                    </Badge>
                  </div>
                  {bot.hfSpaceUrl && (
                    <div className="text-xs flex items-center text-muted-foreground font-mono">
                      <Server className="h-3 w-3 mr-2" />
                      <a href={bot.hfSpaceUrl} target="_blank" rel="noreferrer" className="hover:text-primary hover:underline truncate">
                        {bot.hfSpaceUrl.replace('https://', '')}
                      </a>
                    </div>
                  )}
                </CardContent>
                <CardFooter className="pt-0 flex items-center justify-between border-t border-border/50 mt-auto px-4 py-3 bg-[#0a0a0c]">
                  <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10 h-8 px-2" onClick={() => handleDelete(bot.id)}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                  <Button size="sm" variant="secondary" className="gap-2 h-8" onClick={() => handleDeploy(bot.id)} disabled={deployingId === bot.id}>
                    {deployingId === bot.id ? <Loader2 className="h-3 w-3 animate-spin" /> : <Rocket className="h-3 w-3 text-cyan-400" />}
                    <span className="text-xs">Deploy</span>
                  </Button>
                </CardFooter>
              </Card>
            ))}
          </div>
        )}
      </div>
    </AppLayout>
  );
}
