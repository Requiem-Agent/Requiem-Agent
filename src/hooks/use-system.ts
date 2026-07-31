import { useGetUsage } from "@workspace/api-client-react";

// The 6 free models from OpenCode Zen — hardcoded client-side so no server-side proxy needed.
// API calls to opencode.ai/zen/v1/chat/completions are made directly from the user's browser,
// ensuring requests are counted against the user's IP, not the HF Space IP.
export const FREE_ZEN_MODELS = [
  {
    id: "deepseek-v4-flash-free",
    name: "Deepseek V4 Flash",
    assignedRole: "coder",
    description: "Fast code generation & planning",
    available: true,
  },
  {
    id: "big-pickle",
    name: "Big Pickle",
    assignedRole: "coder",
    description: "Parallel multi-file coding assistant",
    available: true,
  },
  {
    id: "mimo-v2.5-free",
    name: "Mimo V2.5",
    assignedRole: "orchestrator",
    description: "Vision, conversation & task orchestration",
    available: true,
  },
  {
    id: "hy3-free",
    name: "Hy3",
    assignedRole: "planner",
    description: "Heavy reasoning, planning & review",
    available: true,
  },
  {
    id: "north-mini-code-free",
    name: "North Mini Code",
    assignedRole: "reviewer",
    description: "Dependency integrity & code cleanliness",
    available: true,
  },
  {
    id: "nemotron-3-ultra-free",
    name: "Nemotron Ultra",
    assignedRole: "debugger",
    description: "Large tasks, testing & debugging",
    available: true,
  },
] as const;

export type ZenModel = typeof FREE_ZEN_MODELS[number];

// Role → model map
export const ROLE_MODEL_MAP: Record<string, string> = {
  coder:        "deepseek-v4-flash-free",
  orchestrator: "mimo-v2.5-free",
  planner:      "hy3-free",
  reviewer:     "north-mini-code-free",
  debugger:     "nemotron-3-ultra-free",
  designer:     "mimo-v2.5-free",
  researcher:   "hy3-free",
  security:     "deepseek-v4-flash-free",
  explorer:     "big-pickle",
  "back-end":   "deepseek-v4-flash-free",
};

// Static hook — returns the same shape as the old useListModels() so consumers don't change.
export function useModels() {
  return {
    data: {
      models: FREE_ZEN_MODELS as unknown as Array<{
        id: string;
        name: string;
        assignedRole: string;
        description: string;
        available: boolean;
      }>,
      roleMap: ROLE_MODEL_MAP,
    },
    isLoading: false,
    isError: false,
  };
}

export function useUsageStats() {
  return useGetUsage({
    query: {
      staleTime: 1000 * 60 * 5,
    }
  });
}
