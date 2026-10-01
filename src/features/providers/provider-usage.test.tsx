// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { UsageIndicator } from "./provider-usage";
import type { ProviderUsageState } from "@/hooks/use-provider-usage";

afterEach(cleanup);

function state(overrides: Partial<ProviderUsageState> = {}): ProviderUsageState {
  return {
    loading: false,
    refresh: vi.fn().mockResolvedValue(undefined),
    reports: [
      {
        providerId: "codex",
        providerName: "OpenAI Codex CLI",
        usage: {
          providerId: "codex",
          plan: "plus",
          windows: [
            { durationMinutes: 300, usedPercent: 2, resetsAt: null },
            { durationMinutes: 10080, usedPercent: 85, resetsAt: null }
          ],
          limitReached: false,
          checkedAt: new Date().toISOString()
        }
      }
    ],
    ...overrides
  };
}

describe("UsageIndicator", () => {
  it("summarizes usage and opens the breakdown with a fresh read", () => {
    const usage = state();
    render(<UsageIndicator usage={usage} />);
    const trigger = screen.getByRole("button", { name: "Provider usage: 5h 2% · 7d 85%" });

    fireEvent.click(trigger);

    expect(usage.refresh).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("dialog", { name: "Provider usage" })).toBeTruthy();
    const bars = screen.getAllByRole("progressbar");
    expect(bars.map((bar) => bar.getAttribute("aria-label"))).toEqual([
      "5-hour limit",
      "Weekly limit"
    ]);
    expect(bars[1].getAttribute("aria-valuenow")).toBe("85");
  });

  it("closes on Escape and returns focus to the trigger", () => {
    render(<UsageIndicator usage={state()} />);
    const trigger = screen.getByRole("button", { name: /Provider usage/ });
    fireEvent.click(trigger);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("reports unavailable usage with the provider error", () => {
    render(
      <UsageIndicator
        usage={state({
          reports: [
            {
              providerId: "codex",
              providerName: "OpenAI Codex CLI",
              usage: null,
              error: "Codex did not report usage limits"
            }
          ]
        })}
      />
    );
    fireEvent.click(screen.getByRole("button", { name: "Provider usage: Usage unavailable" }));
    expect(screen.getByText("Codex did not report usage limits")).toBeTruthy();
  });
});
