import { useCallback, useEffect, useState } from "react";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { createGroup, deleteGroup, listGroups } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { Group, NewGroupInput } from "@/types/domain";

export interface Groups {
  groups: Group[];
  error?: string;
  /** Resolves to the new group, or undefined with `error` set. */
  create: (input: NewGroupInput) => Promise<Group | undefined>;
  remove: (groupId: string) => Promise<boolean>;
}

/** The user's groups, refreshed on every `group.*` event, including groups agents create. */
export function useGroups(): Groups {
  const [groups, setGroups] = useState<Group[]>([]);
  const [error, setError] = useState<string>();

  const reload = useCallback(async () => {
    try {
      setGroups(await listGroups());
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "Groups could not be loaded."));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useRuntimeEvents((event) => {
    if (event.eventType.startsWith("group.")) void reload();
  });

  const create = useCallback(async (input: NewGroupInput) => {
    try {
      const group = await createGroup(input);
      setGroups((current) =>
        current.some((candidate) => candidate.id === group.id) ? current : [group, ...current]
      );
      setError(undefined);
      return group;
    } catch (caught) {
      setError(describeError(caught, "The group could not be created."));
      return undefined;
    }
  }, []);

  const remove = useCallback(async (groupId: string) => {
    try {
      await deleteGroup(groupId);
      setGroups((current) => current.filter((group) => group.id !== groupId));
      setError(undefined);
      return true;
    } catch (caught) {
      setError(describeError(caught, "The group could not be deleted."));
      return false;
    }
  }, []);

  return { groups, error, create, remove };
}
