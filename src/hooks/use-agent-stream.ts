// use-agent-stream.ts — React WebSocket client hook for real-time agent streaming
// S5-05: Connects to GET /ws/agent and streams tokens from the Requiem Agent backend
//
// Usage:
//   const { send, tokens, steps, status, cancel, reset } = useAgentStream();
//
//   // Start a chat session
//   send({ message: "Explain quantum computing", mode: "chat" });
//
//   // Start an orchestrator session
//   send({ message: "Fix all TypeScript errors", mode: "orchestrator", maxSteps: 15 });
//
//   // Cancel in-flight
//   cancel();
//
// Features:
//   - Auto-reconnect with exponential backoff (max 5 retries)
//   - Cancellation support
//   - Typed message protocol matching ws_agent.rs
//   - Streaming token accumulation
//   - ReAct step tracking
//   - Connection state machine

import { useCallback, useEffect, useRef, useState } from "react";

// ─────────────────────────────────────────────────────────────────────────────
// Types — mirror the Rust ServerMessage enum in ws_agent.rs
// ─────────────────────────────────────────────────────────────────────────────

export type AgentMode = "chat" | "orchestrator" | "code";

export interface StartPayload {
  message: string;
  mode?: AgentMode;
  workspaceId?: string;
  maxSteps?: number;
}

/** Messages sent from client → server */
type ClientMessage =
  | { type: "start"; message: string; mode: AgentMode; workspace_id?: string; max_steps?: number }
  | { type: "cancel" }
  | { type: "ping" };

/** Messages received from server → client */
type ServerMessage =
  | { type: "token"; content: string }
  | { type: "step"; step: number; thought: string }
  | { type: "tool_call"; name: string; args: Record<string, unknown> }
  | { type: "tool_result"; name: string; output: string }
  | { type: "done"; content: string; steps: number }
  | { type: "error"; message: string }
  | { type: "pong" };

/** A single ReAct step visible to the UI */
export interface AgentStep {
  step: number;
  thought: string;
  toolCall?: { name: string; args: Record<string, unknown> };
  toolResult?: { name: string; output: string };
}

/** Connection state machine */
export type StreamStatus =
  | "idle"        // not connected, no session
  | "connecting"  // WebSocket handshake in progress
  | "streaming"   // receiving tokens / steps
  | "done"        // session completed successfully
  | "error"       // session ended with error
  | "cancelled";  // user cancelled

// ─────────────────────────────────────────────────────────────────────────────
// Hook return type
// ─────────────────────────────────────────────────────────────────────────────

export interface UseAgentStreamReturn {
  /** Start a new agent session */
  send: (payload: StartPayload) => void;
  /** Cancel the current session */
  cancel: () => void;
  /** Reset all state back to idle */
  reset: () => void;
  /** Accumulated streaming text (all tokens concatenated) */
  tokens: string;
  /** ReAct steps received so far */
  steps: AgentStep[];
  /** Final complete response (set when done) */
  finalContent: string;
  /** Current connection/session status */
  status: StreamStatus;
  /** Last error message (if status === "error") */
  error: string | null;
  /** Whether a session is currently active */
  isStreaming: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

const WS_URL =
  (import.meta.env.VITE_WS_URL as string | undefined) ??
  (typeof window !== "undefined"
    ? `${window.location.protocol === "https:" ? "wss" : "ws"}://${window.location.host}/ws/agent`
    : "ws://localhost:3000/ws/agent");

const MAX_RETRIES = 5;
const BASE_RETRY_DELAY_MS = 500;
const PING_INTERVAL_MS = 30_000;

// ─────────────────────────────────────────────────────────────────────────────
// Hook implementation
// ─────────────────────────────────────────────────────────────────────────────

export function useAgentStream(): UseAgentStreamReturn {
  const [tokens, setTokens] = useState<string>("");
  const [steps, setSteps] = useState<AgentStep[]>([]);
  const [finalContent, setFinalContent] = useState<string>("");
  const [status, setStatus] = useState<StreamStatus>("idle");
  const [error, setError] = useState<string | null>(null);

  // Internal refs — don't trigger re-renders
  const wsRef = useRef<WebSocket | null>(null);
  const pendingPayloadRef = useRef<StartPayload | null>(null);
  const retryCountRef = useRef<number>(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const cancelledRef = useRef<boolean>(false);
  const currentStepRef = useRef<Partial<AgentStep>>({});

  // ── Cleanup helpers ──────────────────────────────────────────────────────

  const clearTimers = useCallback(() => {
    if (retryTimerRef.current) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
    if (pingTimerRef.current) {
      clearInterval(pingTimerRef.current);
      pingTimerRef.current = null;
    }
  }, []);

  const closeSocket = useCallback(() => {
    clearTimers();
    if (wsRef.current) {
      wsRef.current.onopen = null;
      wsRef.current.onmessage = null;
      wsRef.current.onerror = null;
      wsRef.current.onclose = null;
      if (wsRef.current.readyState === WebSocket.OPEN) {
        wsRef.current.close(1000, "Client closed");
      }
      wsRef.current = null;
    }
  }, [clearTimers]);

  // ── Send a raw message to the server ────────────────────────────────────

  const sendRaw = useCallback((msg: ClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    }
  }, []);

  // ── Handle incoming server messages ─────────────────────────────────────

  const handleMessage = useCallback((event: MessageEvent) => {
    let msg: ServerMessage;
    try {
      msg = JSON.parse(event.data as string) as ServerMessage;
    } catch {
      console.warn("[useAgentStream] Failed to parse server message:", event.data);
      return;
    }

    switch (msg.type) {
      case "token":
        setTokens((prev) => prev + msg.content);
        break;

      case "step": {
        // Flush any pending step and start a new one
        const newStep: AgentStep = { step: msg.step, thought: msg.thought };
        currentStepRef.current = newStep;
        setSteps((prev) => {
          const existing = prev.findIndex((s) => s.step === msg.step);
          if (existing >= 0) {
            const updated = [...prev];
            updated[existing] = { ...updated[existing], thought: msg.thought };
            return updated;
          }
          return [...prev, newStep];
        });
        break;
      }

      case "tool_call":
        setSteps((prev) => {
          if (prev.length === 0) return prev;
          const updated = [...prev];
          const last = { ...updated[updated.length - 1] };
          last.toolCall = { name: msg.name, args: msg.args };
          updated[updated.length - 1] = last;
          return updated;
        });
        break;

      case "tool_result":
        setSteps((prev) => {
          if (prev.length === 0) return prev;
          const updated = [...prev];
          const last = { ...updated[updated.length - 1] };
          last.toolResult = { name: msg.name, output: msg.output };
          updated[updated.length - 1] = last;
          return updated;
        });
        break;

      case "done":
        setFinalContent(msg.content);
        setStatus("done");
        closeSocket();
        break;

      case "error":
        if (!cancelledRef.current) {
          setError(msg.message);
          setStatus("error");
        }
        closeSocket();
        break;

      case "pong":
        // Keepalive acknowledged — no action needed
        break;

      default:
        console.warn("[useAgentStream] Unknown message type:", (msg as { type: string }).type);
    }
  }, [closeSocket]);

  // ── Connect to WebSocket ─────────────────────────────────────────────────

  const connect = useCallback((payload: StartPayload) => {
    closeSocket();
    setStatus("connecting");

    // Auth token as query param — browsers cannot send Authorization headers on WS upgrade
    const _wsToken = sessionStorage.getItem('rq_tok') || localStorage.getItem('requiem_token') || '';
    const _wsUrl = _wsToken ? `${WS_URL}?token=${encodeURIComponent(_wsToken)}` : WS_URL;
    const ws = new WebSocket(_wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      retryCountRef.current = 0;
      setStatus("streaming");

      // Send the start message
      const startMsg: ClientMessage = {
        type: "start",
        message: payload.message,
        mode: payload.mode ?? "chat",
        workspace_id: payload.workspaceId,
        max_steps: payload.maxSteps ?? 10,
      };
      ws.send(JSON.stringify(startMsg));

      // Start keepalive pings
      pingTimerRef.current = setInterval(() => {
        sendRaw({ type: "ping" });
      }, PING_INTERVAL_MS);
    };

    ws.onmessage = handleMessage;

    ws.onerror = (event) => {
      console.error("[useAgentStream] WebSocket error:", event);
    };

    ws.onclose = (event) => {
      clearTimers();

      if (cancelledRef.current) {
        setStatus("cancelled");
        return;
      }

      // Abnormal close — attempt retry with exponential backoff
      if (
        event.code !== 1000 &&
        event.code !== 1001 &&
        retryCountRef.current < MAX_RETRIES &&
        pendingPayloadRef.current
      ) {
        const delay = BASE_RETRY_DELAY_MS * Math.pow(2, retryCountRef.current);
        retryCountRef.current += 1;
        console.warn(
          `[useAgentStream] Connection closed (${event.code}). Retrying in ${delay}ms (attempt ${retryCountRef.current}/${MAX_RETRIES})`
        );
        retryTimerRef.current = setTimeout(() => {
          if (pendingPayloadRef.current) {
            connect(pendingPayloadRef.current);
          }
        }, delay);
      } else if (status !== "done" && status !== "cancelled") {
        setError(`Connection closed: ${event.reason || event.code}`);
        setStatus("error");
      }
    };
  }, [closeSocket, clearTimers, handleMessage, sendRaw, status]);

  // ── Public API ───────────────────────────────────────────────────────────

  const send = useCallback((payload: StartPayload) => {
    // Reset state for new session
    cancelledRef.current = false;
    retryCountRef.current = 0;
    pendingPayloadRef.current = payload;
    currentStepRef.current = {};
    setTokens("");
    setSteps([]);
    setFinalContent("");
    setError(null);
    setStatus("idle");

    connect(payload);
  }, [connect]);

  const cancel = useCallback(() => {
    cancelledRef.current = true;
    sendRaw({ type: "cancel" });
    // Give the server 200ms to acknowledge, then force-close
    setTimeout(() => {
      closeSocket();
      setStatus("cancelled");
    }, 200);
  }, [sendRaw, closeSocket]);

  const reset = useCallback(() => {
    cancelledRef.current = true;
    closeSocket();
    pendingPayloadRef.current = null;
    retryCountRef.current = 0;
    setTokens("");
    setSteps([]);
    setFinalContent("");
    setError(null);
    setStatus("idle");
  }, [closeSocket]);

  // ── Cleanup on unmount ───────────────────────────────────────────────────

  useEffect(() => {
    return () => {
      cancelledRef.current = true;
      closeSocket();
    };
  }, [closeSocket]);

  // ── Derived state ────────────────────────────────────────────────────────

  const isStreaming = status === "connecting" || status === "streaming";

  return {
    send,
    cancel,
    reset,
    tokens,
    steps,
    finalContent,
    status,
    error,
    isStreaming,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience re-export: a simpler hook for pure chat (no steps)
// ─────────────────────────────────────────────────────────────────────────────

export interface UseChatStreamReturn {
  sendMessage: (message: string) => void;
  cancel: () => void;
  reset: () => void;
  content: string;
  isStreaming: boolean;
  isDone: boolean;
  error: string | null;
}

/**
 * Simplified hook for chat-only streaming (no ReAct steps).
 *
 * @example
 * ```tsx
 * const { sendMessage, content, isStreaming } = useChatStream();
 * return (
 *   <div>
 *     <button onClick={() => sendMessage("Hello!")}>Send</button>
 *     <pre>{content}</pre>
 *     {isStreaming && <Spinner />}
 *   </div>
 * );
 * ```
 */
export function useChatStream(): UseChatStreamReturn {
  const { send, cancel, reset, tokens, finalContent, status, error, isStreaming } =
    useAgentStream();

  const sendMessage = useCallback(
    (message: string) => send({ message, mode: "chat" }),
    [send]
  );

  return {
    sendMessage,
    cancel,
    reset,
    content: tokens || finalContent,
    isStreaming,
    isDone: status === "done",
    error,
  };
}
