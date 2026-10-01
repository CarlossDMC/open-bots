import { useCallback, useEffect, useState } from "react";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { createTask, isTauriRuntime, listTasks, updateTaskStatus } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { AgentTask, NewTaskInput, TaskStatus } from "@/types/domain";

export interface Tasks {
  tasks: AgentTask[];
  available: boolean;
  loading: boolean;
  error?: string;
  createError?: string;
  create: (input: NewTaskInput) => Promise<boolean>;
  clearCreateError: () => void;
  setStatus: (task: AgentTask, status: TaskStatus) => Promise<void>;
}

export function useTasks(): Tasks {
  const [tasks, setTasks] = useState<AgentTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [createError, setCreateError] = useState<string>();

  const reload = useCallback(async () => {
    try {
      setTasks(await listTasks());
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "Tasks could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  // Agents create and update tasks in the background.
  useRuntimeEvents((event) => {
    if (event.eventType.startsWith("task.")) void reload();
  });

  const create = useCallback(async (input: NewTaskInput) => {
    try {
      const task = await createTask(input);
      setTasks((current) =>
        current.some((candidate) => candidate.id === task.id) ? current : [task, ...current]
      );
      setCreateError(undefined);
      return true;
    } catch (caught) {
      setCreateError(describeError(caught, "The task could not be created."));
      return false;
    }
  }, []);

  const clearCreateError = useCallback(() => setCreateError(undefined), []);

  const setStatus = useCallback(async (task: AgentTask, status: TaskStatus) => {
    try {
      const updated = await updateTaskStatus(task.id, status);
      setTasks((current) =>
        current.map((candidate) => (candidate.id === updated.id ? updated : candidate))
      );
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The task could not be updated."));
    }
  }, []);

  return {
    tasks,
    available: isTauriRuntime(),
    loading,
    error,
    createError,
    create,
    clearCreateError,
    setStatus
  };
}
