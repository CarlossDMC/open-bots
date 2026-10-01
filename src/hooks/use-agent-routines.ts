import { useCallback, useEffect, useState } from "react";
import {
  createRoutine,
  deleteRoutine,
  isTauriRuntime,
  listRoutines,
  setRoutineEnabled
} from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { NewRoutineInput, Routine } from "@/types/domain";

export interface AgentRoutines {
  routines: Routine[];
  available: boolean;
  loading: boolean;
  error?: string;
  create: (input: NewRoutineInput) => Promise<boolean>;
  setEnabled: (routine: Routine, enabled: boolean) => Promise<void>;
  remove: (routine: Routine) => Promise<void>;
  /** Re-reads routines, e.g. after the scheduler fires one. */
  reload: () => Promise<void>;
}

export function useAgentRoutines(agentId: string): AgentRoutines {
  const [routines, setRoutines] = useState<Routine[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();

  const reload = useCallback(async () => {
    try {
      setRoutines(await listRoutines(agentId));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "Routines could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, [agentId]);

  useEffect(() => {
    setLoading(true);
    void reload();
  }, [reload]);

  const create = useCallback(async (input: NewRoutineInput) => {
    try {
      const routine = await createRoutine(input);
      setRoutines((current) => [...current, routine]);
      setError(undefined);
      return true;
    } catch (caught) {
      setError(describeError(caught, "The routine could not be saved."));
      return false;
    }
  }, []);

  const setEnabled = useCallback(async (routine: Routine, enabled: boolean) => {
    try {
      const updated = await setRoutineEnabled(routine.id, enabled);
      setRoutines((current) =>
        current.map((candidate) => (candidate.id === updated.id ? updated : candidate))
      );
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The routine could not be updated."));
    }
  }, []);

  const remove = useCallback(async (routine: Routine) => {
    try {
      await deleteRoutine(routine.id);
      setRoutines((current) => current.filter((candidate) => candidate.id !== routine.id));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The routine could not be deleted."));
    }
  }, []);

  return {
    routines,
    available: isTauriRuntime(),
    loading,
    error,
    create,
    setEnabled,
    remove,
    reload
  };
}
