// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { usageRefreshDelayMs, useProviderUsage } from "./use-provider-usage";
import { onRuntimeEvent, readProviderUsage } from "@/lib/desktop-api";
import type { RuntimeEvent } from "@/types/domain";

vi.mock("@/lib/desktop-api", () => ({
  onRuntimeEvent: vi.fn(),
  readProviderUsage: vi.fn()
}));

let emit: (event: RuntimeEvent) => void = () => undefined;

function runtimeEvent(eventType: string): RuntimeEvent {
  return { id: eventType, eventType, payload: {}, occurredAt: "2026-10-01T14:00:00.000Z" };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.mocked(readProviderUsage).mockReset().mockResolvedValue([]);
  vi.mocked(onRuntimeEvent).mockImplementation((handler) => {
    emit = handler;
    return Promise.resolve(() => undefined);
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useProviderUsage", () => {
  it("reads usage once when mounted", async () => {
    const { result } = renderHook(() => useProviderUsage());
    await act(() => vi.runOnlyPendingTimersAsync());
    expect(readProviderUsage).toHaveBeenCalledTimes(1);
    expect(result.current.loading).toBe(false);
  });

  it("refreshes once after turns end, ignoring other events", async () => {
    renderHook(() => useProviderUsage());
    await act(() => vi.runOnlyPendingTimersAsync());
    act(() => {
      emit(runtimeEvent("message.created"));
      emit(runtimeEvent("agent.completed"));
      emit(runtimeEvent("agent.failed"));
    });
    await act(() => vi.advanceTimersByTimeAsync(usageRefreshDelayMs - 1));
    expect(readProviderUsage).toHaveBeenCalledTimes(1);
    await act(() => vi.advanceTimersByTimeAsync(1));
    expect(readProviderUsage).toHaveBeenCalledTimes(2);
  });

  it("keeps the error when a read fails", async () => {
    vi.mocked(readProviderUsage).mockRejectedValue(new Error("Codex app-server timed out"));
    const { result } = renderHook(() => useProviderUsage());
    await act(() => vi.runOnlyPendingTimersAsync());
    expect(result.current.error).toBe("Codex app-server timed out");
  });
});
