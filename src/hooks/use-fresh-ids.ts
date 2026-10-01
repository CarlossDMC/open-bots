import { useState } from "react";

/**
 * Returns the ids that appeared after the list first became ready, so items loaded with the
 * initial history render in place and only new arrivals animate.
 */
export function useFreshIds(ids: string[], ready: boolean): Set<string> {
  const [baseline, setBaseline] = useState<Set<string> | null>(null);
  if (ready && baseline === null) {
    // Adjusting state during render is React's recommended way to derive from a transition.
    setBaseline(new Set(ids));
  }
  if (!ready || baseline === null) return new Set();
  return new Set(ids.filter((id) => !baseline.has(id)));
}
