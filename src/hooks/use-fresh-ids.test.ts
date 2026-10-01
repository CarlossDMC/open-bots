// @vitest-environment jsdom
import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useFreshIds } from "./use-fresh-ids";

describe("useFreshIds", () => {
  it("treats the history loaded first as settled and later ids as fresh", () => {
    const { result, rerender } = renderHook(({ ids, ready }) => useFreshIds(ids, ready), {
      initialProps: { ids: [] as string[], ready: false }
    });
    expect(result.current.size).toBe(0);

    rerender({ ids: ["a", "b"], ready: true });
    expect([...result.current]).toEqual([]);

    rerender({ ids: ["a", "b", "c"], ready: true });
    expect([...result.current]).toEqual(["c"]);
  });

  it("does not mark anything fresh before the list is ready", () => {
    const { result } = renderHook(() => useFreshIds(["a"], false));
    expect(result.current.size).toBe(0);
  });
});
