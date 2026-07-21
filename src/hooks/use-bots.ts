import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { 
  useListBots, 
  useCreateBot, 
  useGetBot, 
  useDeleteBot, 
  useDeployBot,
  getListBotsQueryKey,
  getGetBotQueryKey,
  BotInput
} from "@workspace/api-client-react";

const API_BASE = import.meta.env.VITE_API_URL || "";

function getToken(): string {
  return localStorage.getItem("requiem_token") || "";
}

async function apiFetch(path: string, opts: RequestInit = {}) {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${getToken()}`,
    ...((opts.headers as Record<string, string>) || {}),
  };
  const res = await fetch(`${API_BASE}/api${path}`, { ...opts, headers });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export function useBots() {
  return useListBots({
    query: {
      staleTime: 1000 * 60 * 5, // 5 mins
    }
  });
}

export function useBot(id: string) {
  return useGetBot(id, {
    query: {
      enabled: !!id,
    }
  });
}

export function useBotMutations() {
  const queryClient = useQueryClient();
  const createMutation = useCreateBot();
  const deleteMutation = useDeleteBot();
  const deployMutation = useDeployBot();

  const create = async (data: BotInput) => {
    const result = await createMutation.mutateAsync({ data });
    queryClient.invalidateQueries({ queryKey: getListBotsQueryKey() });
    return result;
  };

  const remove = async (id: string) => {
    await deleteMutation.mutateAsync({ id });
    queryClient.invalidateQueries({ queryKey: getListBotsQueryKey() });
  };

  const deploy = async (id: string) => {
    const result = await deployMutation.mutateAsync({ id });
    queryClient.invalidateQueries({ queryKey: getGetBotQueryKey(id) });
    queryClient.invalidateQueries({ queryKey: getListBotsQueryKey() });
    return result;
  };

  // ── New managed-bot mutations ──────────────────────────────────────────────

  const provision = useMutation({
    mutationFn: (data: { name: string; description?: string; purpose?: string }) =>
      apiFetch("/bots/provision", {
        method: "POST",
        body: JSON.stringify(data),
        headers: { "Content-Type": "application/json" },
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: getListBotsQueryKey() }),
  });

  const linkToken = useMutation({
    mutationFn: ({ id, token }: { id: string; token: string }) =>
      apiFetch(`/bots/${id}/link-token`, {
        method: "POST",
        body: JSON.stringify({ token }),
        headers: { "Content-Type": "application/json" },
      }),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: getListBotsQueryKey() });
      queryClient.invalidateQueries({ queryKey: getGetBotQueryKey(id) });
    },
  });

  return {
    create,
    remove,
    deploy,
    isCreating: createMutation.isPending,
    isDeleting: deleteMutation.isPending,
    isDeploying: deployMutation.isPending,
    provision,
    linkToken,
  };
}

// ── Status poller — polls every 5s while bot is in a transitional state ───────
const TRANSIENT_STATUSES = new Set(["deploying", "building", "pending"]);

export function useBotStatusPoller(botId: string, active: boolean) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!active || !botId) return;

    const interval = setInterval(() => {
      queryClient.invalidateQueries({ queryKey: getGetBotQueryKey(botId) });
      queryClient.invalidateQueries({ queryKey: getListBotsQueryKey() });
    }, 5000);

    return () => clearInterval(interval);
  }, [botId, active, queryClient]);
}

export { TRANSIENT_STATUSES };
