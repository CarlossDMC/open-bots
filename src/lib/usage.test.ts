import { describe, expect, it } from "vitest";
import {
  formatResetShort,
  formatResetsIn,
  latestCheck,
  primaryWindows,
  providerShortName,
  summarizeUsage,
  windowLevel,
  windowName,
  windowShortLabel
} from "./usage";
import type { ProviderUsage, ProviderUsageReport } from "@/types/domain";

function usage(overrides: Partial<ProviderUsage> = {}): ProviderUsage {
  return {
    providerId: "codex",
    plan: "plus",
    windows: [
      { durationMinutes: 300, usedPercent: 2, resetsAt: null },
      { durationMinutes: 10080, usedPercent: 30, resetsAt: null }
    ],
    limitReached: false,
    checkedAt: "2026-10-01T14:00:00.000Z",
    ...overrides
  };
}

function report(
  overrides: Partial<ProviderUsage> = {},
  providerName = "OpenAI Codex CLI"
): ProviderUsageReport {
  return { providerId: "codex", providerName, usage: usage(overrides) };
}

describe("usage windows", () => {
  it("labels window durations and scopes", () => {
    expect(windowShortLabel(300)).toBe("5h");
    expect(windowShortLabel(10080)).toBe("7d");
    expect(windowShortLabel(30)).toBe("30m");
    expect(windowShortLabel(null)).toBe("Limit");
    expect(windowShortLabel(10080, "Fable")).toBe("7d Fable");
    expect(windowName(300)).toBe("5-hour");
    expect(windowName(10080)).toBe("Weekly");
    expect(windowName(1440)).toBe("Daily");
    expect(windowName(undefined)).toBe("Limit");
    expect(windowName(10080, "Fable")).toBe("Weekly · Fable");
  });

  it("keeps scoped windows out of the primary set unless nothing else exists", () => {
    const scoped = { durationMinutes: 10080, scope: "Fable", usedPercent: 0 };
    const all = { durationMinutes: 10080, usedPercent: 20 };
    expect(primaryWindows(usage({ windows: [all, scoped] }))).toEqual([all]);
    expect(primaryWindows(usage({ windows: [scoped] }))).toEqual([scoped]);
  });

  it("shortens provider names that end in CLI", () => {
    expect(providerShortName("OpenAI Codex CLI")).toBe("OpenAI Codex");
    expect(providerShortName("Claude Code")).toBe("Claude Code");
    expect(providerShortName("CLI")).toBe("CLI");
  });

  it("describes the time until a window resets", () => {
    const now = new Date("2026-10-01T14:00:00.000Z");
    expect(formatResetsIn("2026-10-01T16:14:00.000Z", now)).toBe("Resets in 2h 14m");
    expect(formatResetsIn("2026-10-01T14:09:30.000Z", now)).toBe("Resets in 10m");
    expect(formatResetsIn("2026-10-05T19:10:00.000Z", now)).toBe("Resets in 4d 5h");
    expect(formatResetsIn("2026-10-01T13:00:00.000Z", now)).toBe("Resets now");
    expect(formatResetsIn(null, now)).toBeUndefined();
    expect(formatResetShort("2026-10-01T16:05:00.000Z", now)).toBe("2h 05m");
    expect(formatResetShort("2026-10-05T19:10:00.000Z", now)).toBe("4d 05h");
    expect(formatResetShort("2026-10-01T14:09:30.000Z", now)).toBe("10m");
    expect(formatResetShort("2026-10-01T13:00:00.000Z", now)).toBe("now");
    expect(formatResetShort("not a date", now)).toBeUndefined();
  });

  it("raises the level near and at the limit", () => {
    expect(windowLevel({ usedPercent: 79 })).toBe("normal");
    expect(windowLevel({ usedPercent: 80 })).toBe("warning");
    expect(windowLevel({ usedPercent: 100 })).toBe("danger");
    expect(windowLevel({ usedPercent: 5 }, true)).toBe("danger");
  });
});

describe("summarizeUsage", () => {
  it("names each provider with its primary windows", () => {
    const claude = report(
      {
        windows: [
          { durationMinutes: 300, usedPercent: 14 },
          { durationMinutes: 10080, scope: "Fable", usedPercent: 4 }
        ]
      },
      "Claude Code"
    );
    expect(summarizeUsage([report(), claude])).toEqual({
      text: "OpenAI Codex 5h 2%, 7d 30%; Claude Code 5h 14%",
      level: "normal"
    });
  });

  it("reports the most severe level and skips failed providers", () => {
    const failed: ProviderUsageReport = {
      providerId: "other",
      providerName: "Other",
      usage: null,
      error: "offline"
    };
    expect(
      summarizeUsage([failed, report({ windows: [{ durationMinutes: 300, usedPercent: 91 }] })])
    ).toEqual({ text: "OpenAI Codex 5h 91%", level: "warning" });
    expect(summarizeUsage([report({ windows: [], limitReached: true })])).toEqual({
      text: "OpenAI Codex limit reached",
      level: "danger"
    });
    expect(summarizeUsage([failed])).toBeUndefined();
  });

  it("finds the latest check time", () => {
    expect(
      latestCheck([
        report({ checkedAt: "2026-10-01T14:00:00.000Z" }),
        report({ checkedAt: "2026-10-01T14:05:00.000Z" })
      ])
    ).toBe("2026-10-01T14:05:00.000Z");
    expect(latestCheck([])).toBeUndefined();
  });
});
