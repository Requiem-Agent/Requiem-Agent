import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  useListSessions,
  useCreateSession,
  useGetSession,
  useUpdateSession,
  useDeleteSession,
  useListMessages,
  useAddMessage,
  getListSessionsQueryKey,
  getGetSessionQueryKey,
  getListMessagesQueryKey,
  SessionInput,
  SessionUpdate,
  MessageInput,
} from "@workspace/api-client-react";

export function useSessions() {
  return useListSessions();
}

export function useSession(id: string) {
  return useGetSession(id, {
    query: {
      enabled: !!id,
    }
  });
}

export function useMessages(sessionId: string) {
  return useListMessages(sessionId, {
    query: {
      enabled: !!sessionId,
      staleTime: 30_000,       // 30 s — avoids aggressive re-fetch while streaming
      refetchOnWindowFocus: false,
    }
  });
}

export function useSessionMutations() {
  const queryClient = useQueryClient();
  const createMutation = useCreateSession();
  const updateMutation = useUpdateSession();
  const deleteMutation = useDeleteSession();

  const create = async (data: SessionInput) => {
    const result = await createMutation.mutateAsync({ data });
    queryClient.invalidateQueries({ queryKey: getListSessionsQueryKey() });
    return result;
  };

  const update = async (id: string, data: SessionUpdate) => {
    const result = await updateMutation.mutateAsync({ id, data });
    // Optimistic update of local cache would be nice, but simple invalidate is safer
    queryClient.invalidateQueries({ queryKey: getListSessionsQueryKey() });
    queryClient.invalidateQueries({ queryKey: getGetSessionQueryKey(id) });
    return result;
  };

  const remove = async (id: string) => {
    await deleteMutation.mutateAsync({ id });
    queryClient.invalidateQueries({ queryKey: getListSessionsQueryKey() });
  };

  return {
    create,
    update,
    remove,
    isCreating: createMutation.isPending,
    isUpdating: updateMutation.isPending,
    isDeleting: deleteMutation.isPending,
  };
}

export function useMessageMutations(sessionId: string) {
  const queryClient = useQueryClient();
  const addMutation = useAddMessage();

  /**
   * Add a message.
   * @param data - MessageInput payload
   * @param skipInvalidate - When true the caller owns invalidation timing
   *   (use this while streaming to avoid re-fetching mid-stream).
   */
  const add = async (data: MessageInput, skipInvalidate = false) => {
    const result = await addMutation.mutateAsync({ id: sessionId, data });
    if (!skipInvalidate) {
      queryClient.invalidateQueries({ queryKey: getListMessagesQueryKey(sessionId) });
      queryClient.invalidateQueries({ queryKey: getGetSessionQueryKey(sessionId) });
    }
    return result;
  };

  const invalidateMessages = () => {
    queryClient.invalidateQueries({ queryKey: getListMessagesQueryKey(sessionId) });
    queryClient.invalidateQueries({ queryKey: getGetSessionQueryKey(sessionId) });
  };

  return {
    add,
    invalidateMessages,
    isAdding: addMutation.isPending,
  };
}
