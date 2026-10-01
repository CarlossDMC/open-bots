// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProviderUsageList, UsageIndicator } from "./provider-usage";
import type { ProviderUsageState } from "@/hooks/use-provider-usage";
import type { ProviderUsageReport } from "@/types/domain";

afterEach(cleanup);

const checkedAt = new Date().toISOString();

const claude: ProviderUsageReport = {
  providerId: "claude-code",
  providerName: "Claude Code",
  usage: {
    providerId: "claude-code",
    plan: "team",
    windows: [
      { durationMinutes: 300, usedPercent: 14, resetsAt: null },
      { durationMinutes: 10080, usedPercent: 85, resetsAt: null },
      { durationMinutes: 10080, scope: "Fable", usedPercent: 0, resetsAt: null }
    ],
    limitReached: false,
    checkedAt
  }
};

const codex: ProviderUsageReport = {
  providerId: "codex",
  providerName: "OpenAI Codex CLI",
  usage: {
    providerId: "codex",
    plan: "plus",
    windows: [{ durationMinutes: 300, usedPercent: 2, resetsAt: null }],
    limitReached: false,
    checkedAt
  }
};

function state(overrides: Partial<ProviderUsageState> = {}): ProviderUsageState {
  return {
    loading: false,
    refresh: vi.fn().mockResolvedValue(undefined),
    reports: [claude, codex],
    ...overrides
  };
}

describe("UsageIndicator", () => {
  it("shows one row per provider with its primary windows", () => {
    render(<UsageIndicator usage={state()} />);
    const trigger = screen.getByRole("button", {
      name: "Provider usage: Claude Code 5h 14%, 7d 85%; OpenAI Codex 5h 2%"
    });
    expect(within(trigger).getByText("Claude Code")).toBeTruthy();
    expect(within(trigger).getByText("OpenAI Codex")).toBeTruthy();
    expect(within(trigger).getByText("85%").className).toContain("text-warning-foreground");
    expect(within(trigger).queryByText(/Fable/)).toBeNull();
  });

  it("opens the breakdown with a fresh read, including scoped windows", () => {
    const usage = state();
    render(<UsageIndicator usage={usage} />);
    fireEvent.click(screen.getByRole("button", { name: /Provider usage/ }));

    expect(usage.refresh).toHaveBeenCalledTimes(1);
    const dialog = screen.getByRole("dialog", { name: "Provider usage" });
    const claudeSection = within(dialog).getByRole("region", { name: "Claude Code usage" });
    expect(
      within(claudeSection)
        .getAllByRole("progressbar")
        .map((bar) => bar.getAttribute("aria-label"))
    ).toEqual(["Claude Code 5-hour", "Claude Code Weekly", "Claude Code Weekly · Fable"]);
    expect(within(claudeSection).getByText("team")).toBeTruthy();
  });

  it("closes on Escape and returns focus to the trigger", () => {
    render(<UsageIndicator usage={state()} />);
    const trigger = screen.getByRole("button", { name: /Provider usage/ });
    fireEvent.click(trigger);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("marks failed providers as unavailable and shows the error in the breakdown", () => {
    const failed: ProviderUsageReport = {
      providerId: "codex",
      providerName: "OpenAI Codex CLI",
      usage: null,
      error: "Codex did not report usage limits"
    };
    render(<UsageIndicator usage={state({ reports: [failed] })} />);
    const trigger = screen.getByRole("button", { name: "Provider usage: Usage unavailable" });
    expect(within(trigger).getByText("Unavailable")).toBeTruthy();
    fireEvent.click(trigger);
    expect(screen.getByText("Codex did not report usage limits")).toBeTruthy();
  });

  it("reports loading and empty states", () => {
    const { rerender } = render(<UsageIndicator usage={state({ reports: [], loading: true })} />);
    expect(screen.getByRole("button", { name: "Provider usage: Reading usage…" })).toBeTruthy();
    rerender(<UsageIndicator usage={state({ reports: [] })} />);
    expect(screen.getByRole("button", { name: "Provider usage: No usage reported" })).toBeTruthy();
  });
});

describe("ProviderUsageList", () => {
  it("flags reached limits and offers a refresh when requested", () => {
    const reached: ProviderUsageReport = {
      ...codex,
      usage: {
        ...codex.usage!,
        windows: [{ durationMinutes: 300, usedPercent: 100, resetsAt: null }],
        limitReached: true
      }
    };
    const usage = state({ reports: [reached] });
    render(<ProviderUsageList usage={usage} showRefresh />);
    expect(screen.getByText("Limit reached")).toBeTruthy();
    expect(screen.getByText("100%").className).toContain("text-danger-foreground");
    fireEvent.click(screen.getByRole("button", { name: "Refresh usage" }));
    expect(usage.refresh).toHaveBeenCalledTimes(1);
  });
});
