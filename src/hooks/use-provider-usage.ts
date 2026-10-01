import { useCallback, useEffect, useRef, useState } from "react";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { readProviderUsage } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { ProviderUsageReport } from "@/types/domain";

/** Events that end a provider turn, after which usage has likely changed. */
const turnEndEvents = new Set(["agent.completed", "agent.failed", "agent.cancelled"]);
/** Collapses several turns ending together into one read. */
export const usageRefreshDelayMs = 2_000;

export interface ProviderUsageState {
  reports: ProviderUsageReport[];
  loading: boolean;
  error?: string;
  refresh: () => Promise<void>;
}

/** Reads provider usage on mount, after each turn, and on demand. It never polls. */
export function useProviderUsage(): ProviderUsageState {
  const [reports, setReports] = useState<ProviderUsageReport[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setReports(await readProviderUsage());
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "Provider usage could not be read."));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    return () => clearTimeout(timer.current);
  }, [refresh]);

  useRuntimeEvents((event) => {
    if (!turnEndEvents.has(event.eventType)) return;
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void refresh(), usageRefreshDelayMs);
  });

  return { reports, loading, error, refresh };
}
