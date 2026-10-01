import { describe, expect, it } from "vitest";
import { describeError, formatConversationTime, formatRelativeTime, titleCase } from "./utils";

describe("formatRelativeTime", () => {
  it("formats elapsed minutes and hours", () => {
    const now = new Date("2026-01-01T12:00:00.000Z");
    expect(formatRelativeTime("2026-01-01T11:48:00.000Z", now)).toBe("12m ago");
    expect(formatRelativeTime("2026-01-01T09:00:00.000Z", now)).toBe("3h ago");
  });
});

describe("formatConversationTime", () => {
  const now = new Date(2026, 0, 10, 15, 30);

  it("shows the clock time for today", () => {
    expect(formatConversationTime(new Date(2026, 0, 10, 9, 5).toISOString(), now)).toBe("09:05");
  });

  it("labels the previous calendar day as yesterday", () => {
    expect(formatConversationTime(new Date(2026, 0, 9, 23, 59).toISOString(), now)).toBe(
      "Yesterday"
    );
  });

  it("shows the weekday within the last week", () => {
    expect(formatConversationTime(new Date(2026, 0, 6, 12, 0).toISOString(), now)).toBe("Tue");
  });

  it("shows a short date for older timestamps", () => {
    expect(formatConversationTime(new Date(2025, 11, 20, 12, 0).toISOString(), now)).toBe("Dec 20");
  });
});

describe("titleCase", () => {
  it("formats machine-readable labels", () => {
    expect(titleCase("approval-required")).toBe("Approval Required");
  });
});

describe("describeError", () => {
  it("prefers error messages and command rejection strings", () => {
    expect(describeError(new Error("disk full"), "fallback")).toBe("disk full");
    expect(describeError("resource not found: agent", "fallback")).toBe(
      "resource not found: agent"
    );
  });

  it("falls back for empty or unknown values", () => {
    expect(describeError("", "fallback")).toBe("fallback");
    expect(describeError({ code: 1 }, "fallback")).toBe("fallback");
  });
});
