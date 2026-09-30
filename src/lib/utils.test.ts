import { describe, expect, it } from "vitest";
import { formatRelativeTime, titleCase } from "./utils";

describe("formatRelativeTime", () => {
  it("formats elapsed minutes and hours", () => {
    const now = new Date("2026-01-01T12:00:00.000Z");
    expect(formatRelativeTime("2026-01-01T11:48:00.000Z", now)).toBe("12m ago");
    expect(formatRelativeTime("2026-01-01T09:00:00.000Z", now)).toBe("3h ago");
  });
});

describe("titleCase", () => {
  it("formats machine-readable labels", () => {
    expect(titleCase("approval-required")).toBe("Approval Required");
  });
});
