import { describe, expect, it } from "vitest";
import {
  formatResetsIn,
  summarizeUsage,
  windowLevel,
  windowLongLabel,
  windowShortLabel
} from "./usage";
import type { ProviderUsageReport } from "@/types/domain";

function report(overrides: Partial<NonNullable<ProviderUsageReport["usage"]>> = {}) {
  return {
    providerId: "codex",
    providerName: "OpenAI Codex CLI",
    usage: {
      providerId: "codex",
      plan: "plus",
      windows: [
        { durationMinutes: 300, usedPercent: 2, resetsAt: null },
        { durationMinutes: 10080, usedPercent: 30, resetsAt: null }
      ],
      limitReached: false,
      checkedAt: "2026-10-01T14:00:00.000Z",
      ...overrides
    }
  } satisfies ProviderUsageReport;
}

describe("usage windows", () => {
  it("labels window durations", () => {
    expect(windowShortLabel(300)).toBe("5h");
    expect(windowShortLabel(10080)).toBe("7d");
    expect(windowShortLabel(30)).toBe("30m");
    expect(windowShortLabel(null)).toBe("Limit");
    expect(windowLongLabel(300)).toBe("5-hour limit");
    expect(windowLongLabel(10080)).toBe("Weekly limit");
    expect(windowLongLabel(1440)).toBe("Daily limit");
    expect(windowLongLabel(undefined)).toBe("Usage limit");
  });

  it("describes the time until a window resets", () => {
    const now = new Date("2026-10-01T14:00:00.000Z");
    expect(formatResetsIn("2026-10-01T16:14:00.000Z", now)).toBe("Resets in 2h 14m");
    expect(formatResetsIn("2026-10-01T14:09:30.000Z", now)).toBe("Resets in 10m");
    expect(formatResetsIn("2026-10-05T19:10:00.000Z", now)).toBe("Resets in 4d 5h");
    expect(formatResetsIn("2026-10-01T13:00:00.000Z", now)).toBe("Resets now");
    expect(formatResetsIn(null, now)).toBeUndefined();
  });

  it("raises the level near and at the limit", () => {
    expect(windowLevel({ usedPercent: 79 })).toBe("normal");
    expect(windowLevel({ usedPercent: 80 })).toBe("warning");
    expect(windowLevel({ usedPercent: 100 })).toBe("danger");
    expect(windowLevel({ usedPercent: 5 }, true)).toBe("danger");
  });
});

describe("summarizeUsage", () => {
  it("joins every window into one line", () => {
    expect(summarizeUsage([report()])).toEqual({ text: "5h 2% · 7d 30%", level: "normal" });
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
    ).toEqual({ text: "5h 91%", level: "warning" });
    expect(summarizeUsage([report({ windows: [], limitReached: true })])).toEqual({
      text: "Limit reached",
      level: "danger"
    });
    expect(summarizeUsage([failed])).toBeUndefined();
  });
});
