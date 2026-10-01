import { useEffect, useRef } from "react";
import { onRuntimeEvent } from "@/lib/desktop-api";
import type { RuntimeEvent } from "@/types/domain";

/** Calls `handler` for every live runtime event while the component is mounted. */
export function useRuntimeEvents(handler: (event: RuntimeEvent) => void): void {
  const latestHandler = useRef(handler);
  useEffect(() => {
    latestHandler.current = handler;
  });

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void onRuntimeEvent((event) => latestHandler.current(event)).then((stop) => {
      if (active) unsubscribe = stop;
      else stop();
    });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, []);
}
