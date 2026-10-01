import { describe, expect, it } from "vitest";
import { describeSchedule, formatNextRun, parseScheduleForm } from "./routines";

describe("describeSchedule", () => {
  it("describes intervals in the largest whole unit", () => {
    expect(describeSchedule({ kind: "interval", minutes: 30 })).toBe("Every 30 min");
    expect(describeSchedule({ kind: "interval", minutes: 60 })).toBe("Every hour");
    expect(describeSchedule({ kind: "interval", minutes: 180 })).toBe("Every 3 hours");
    expect(describeSchedule({ kind: "interval", minutes: 1440 })).toBe("Every day");
    expect(describeSchedule({ kind: "interval", minutes: 4320 })).toBe("Every 3 days");
  });

  it("describes daily times with padded clock values", () => {
    expect(describeSchedule({ kind: "daily", hour: 9, minute: 5 })).toBe("Daily at 09:05");
  });
});

describe("parseScheduleForm", () => {
  it("parses valid intervals and daily times", () => {
    expect(parseScheduleForm("interval", "45")).toEqual({
      schedule: { kind: "interval", minutes: 45 }
    });
    expect(parseScheduleForm("daily", "07:30")).toEqual({
      schedule: { kind: "daily", hour: 7, minute: 30 }
    });
  });

  it("rejects out-of-range or malformed values", () => {
    for (const value of ["4", "10081", "12.5", "", "abc"]) {
      expect(parseScheduleForm("interval", value)).toHaveProperty("error");
    }
    for (const value of ["24:00", "9:00", "12:60", ""]) {
      expect(parseScheduleForm("daily", value)).toHaveProperty("error");
    }
  });
});

describe("formatNextRun", () => {
  const now = new Date(2026, 0, 10, 15, 30);

  it("labels today and tomorrow", () => {
    expect(formatNextRun(new Date(2026, 0, 10, 18, 0).toISOString(), now)).toBe("Today 18:00");
    expect(formatNextRun(new Date(2026, 0, 11, 9, 5).toISOString(), now)).toBe("Tomorrow 09:05");
  });

  it("shows a short date further out", () => {
    expect(formatNextRun(new Date(2026, 0, 14, 9, 0).toISOString(), now)).toBe("Jan 14 09:00");
  });
});
