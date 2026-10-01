import { useCallback, useEffect, useState } from "react";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import {
  cancelTurn,
  clearConversation,
  listMessages,
  resetAgentSession,
  sendMessage
} from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { ConversationMessage, RuntimeEvent } from "@/types/domain";

export interface Conversation {
  messages: ConversationMessage[];
  loading: boolean;
  error?: string;
  /** The action the agent is running right now, from live `tool.*` events. */
  currentAction?: string;
  send: (content: string) => Promise<boolean>;
  cancel: () => Promise<void>;
  /** Starts a fresh provider session; resolves to false and sets `error` on failure. */
  resetSession: () => Promise<boolean>;
  /** Deletes every message and starts fresh; resolves to false and sets `error` on failure. */
  clear: () => Promise<boolean>;
}

const turnEndEvents = new Set(["agent.completed", "agent.failed", "agent.cancelled"]);

export function useConversation(agentId: string): Conversation {
  const [messages, setMessages] = useState<ConversationMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [currentAction, setCurrentAction] = useState<string>();

  const reload = useCallback(async () => {
    try {
      setMessages(await listMessages(agentId));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The conversation could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, [agentId]);

  useEffect(() => {
    setLoading(true);
    setCurrentAction(undefined);
    void reload();
  }, [reload]);

  useRuntimeEvents((event: RuntimeEvent) => {
    const eventAgentId = event.payload.agentId ?? event.aggregateId;
    if (eventAgentId !== agentId) return;
    if (event.eventType === "message.created" || event.eventType === "agent.conversation_cleared")
      void reload();
    // Actions in a group turn are shown in the group.
    else if (event.payload.groupId) return;
    else if (event.eventType === "tool.started" && typeof event.payload.detail === "string") {
      setCurrentAction(event.payload.detail);
    } else if (event.eventType === "tool.completed" || event.eventType === "tool.failed") {
      setCurrentAction(undefined);
    } else if (turnEndEvents.has(event.eventType)) setCurrentAction(undefined);
  });

  const send = useCallback(
    async (content: string) => {
      try {
        const message = await sendMessage(agentId, content);
        setMessages((current) =>
          current.some((candidate) => candidate.id === message.id) ? current : [...current, message]
        );
        setError(undefined);
        return true;
      } catch (caught) {
        setError(describeError(caught, "The message could not be sent."));
        return false;
      }
    },
    [agentId]
  );

  const cancel = useCallback(async () => {
    try {
      await cancelTurn(agentId);
    } catch (caught) {
      setError(describeError(caught, "The turn could not be stopped."));
    }
  }, [agentId]);

  const resetSession = useCallback(async () => {
    try {
      await resetAgentSession(agentId);
      setError(undefined);
      return true;
    } catch (caught) {
      setError(describeError(caught, "A new session could not be started."));
      return false;
    }
  }, [agentId]);

  const clear = useCallback(async () => {
    try {
      await clearConversation(agentId);
      setMessages([]);
      setError(undefined);
      return true;
    } catch (caught) {
      setError(describeError(caught, "The conversation could not be cleared."));
      return false;
    }
  }, [agentId]);

  return { messages, loading, error, currentAction, send, cancel, resetSession, clear };
}
