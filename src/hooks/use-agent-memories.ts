import { useCallback, useEffect, useState } from "react";
import { addMemory, listMemories, removeMemory } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { AgentMemory } from "@/types/domain";

export interface AgentMemories {
  memories: AgentMemory[];
  loading: boolean;
  error?: string;
  add: (content: string) => Promise<boolean>;
  remove: (memory: AgentMemory) => Promise<void>;
  reload: () => Promise<void>;
}

export function useAgentMemories(agentId: string): AgentMemories {
  const [memories, setMemories] = useState<AgentMemory[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      setMemories(await listMemories(agentId));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "Memories could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, [agentId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const add = useCallback(
    async (content: string) => {
      try {
        const memory = await addMemory(agentId, content);
        setMemories((current) => [memory, ...current]);
        setError(undefined);
        return true;
      } catch (caught) {
        setError(describeError(caught, "The memory could not be saved."));
        return false;
      }
    },
    [agentId]
  );

  const remove = useCallback(async (memory: AgentMemory) => {
    try {
      await removeMemory(memory);
      setMemories((current) => current.filter((candidate) => candidate.id !== memory.id));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The memory could not be removed."));
    }
  }, []);

  return { memories, loading, error, add, remove, reload };
}
