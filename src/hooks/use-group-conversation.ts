import { useCallback, useEffect, useState } from "react";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { listGroupMessages, sendGroupMessage, stopGroup } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { GroupMessage, RuntimeEvent } from "@/types/domain";

export interface GroupConversation {
  messages: GroupMessage[];
  loading: boolean;
  error?: string;
  /** The action the answering member is running, from live `tool.*` events in this group. */
  currentAction?: string;
  send: (content: string) => Promise<boolean>;
  stop: () => Promise<void>;
}

const turnEndEvents = new Set(["agent.completed", "agent.failed", "agent.cancelled"]);

export function useGroupConversation(groupId: string): GroupConversation {
  const [messages, setMessages] = useState<GroupMessage[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [currentAction, setCurrentAction] = useState<string>();

  const reload = useCallback(async () => {
    try {
      setMessages(await listGroupMessages(groupId));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The group could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, [groupId]);

  useEffect(() => {
    setLoading(true);
    setCurrentAction(undefined);
    void reload();
  }, [reload]);

  useRuntimeEvents((event: RuntimeEvent) => {
    if (event.payload.groupId !== groupId) return;
    if (event.eventType === "group.message_created") void reload();
    else if (event.eventType === "tool.started" && typeof event.payload.detail === "string") {
      setCurrentAction(event.payload.detail);
    } else if (event.eventType === "tool.completed" || event.eventType === "tool.failed") {
      setCurrentAction(undefined);
    } else if (turnEndEvents.has(event.eventType)) setCurrentAction(undefined);
  });

  const send = useCallback(
    async (content: string) => {
      try {
        const message = await sendGroupMessage(groupId, content);
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
    [groupId]
  );

  const stop = useCallback(async () => {
    try {
      await stopGroup(groupId);
    } catch (caught) {
      setError(describeError(caught, "The group could not be stopped."));
    }
  }, [groupId]);

  return { messages, loading, error, currentAction, send, stop };
}
