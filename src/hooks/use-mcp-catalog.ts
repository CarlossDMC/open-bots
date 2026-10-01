import { useCallback, useEffect, useState } from "react";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { listMcpCatalog, saveMcpCatalog } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { McpCatalogEntry } from "@/types/domain";

export interface McpCatalog {
  entries: McpCatalogEntry[];
  loading: boolean;
  error?: string;
  /** Replaces one provider's entries; resolves to false and sets `error` on failure. */
  save: (providerId: string, servers: string[]) => Promise<boolean>;
}

/** The global MCP server catalog, kept current with `mcp_catalog.updated` events. */
export function useMcpCatalog(): McpCatalog {
  const [entries, setEntries] = useState<McpCatalogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();

  const reload = useCallback(async () => {
    try {
      setEntries(await listMcpCatalog());
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The MCP server catalog could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useRuntimeEvents((event) => {
    if (event.eventType === "mcp_catalog.updated") void reload();
  });

  const save = useCallback(async (providerId: string, servers: string[]) => {
    try {
      setEntries(await saveMcpCatalog(providerId, servers));
      setError(undefined);
      return true;
    } catch (caught) {
      setError(describeError(caught, "The MCP server catalog could not be saved."));
      return false;
    }
  }, []);

  return { entries, loading, error, save };
}
