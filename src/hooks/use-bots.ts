import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
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

  return {
    create,
    remove,
    deploy,
    isCreating: createMutation.isPending,
    isDeleting: deleteMutation.isPending,
    isDeploying: deployMutation.isPending,
  };
}
