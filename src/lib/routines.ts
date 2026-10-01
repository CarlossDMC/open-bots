import type { RoutineSchedule } from "@/types/domain";

/** Mirrors the limits in `src-tauri/src/domain/routines.rs`. */
export const minIntervalMinutes = 5;
export const maxIntervalMinutes = 7 * 24 * 60;
export const maxRoutinesPerAgent = 50;

export type ScheduleKind = RoutineSchedule["kind"];

export function describeSchedule(schedule: RoutineSchedule): string {
  if (schedule.kind === "daily") {
    return `Daily at ${pad(schedule.hour)}:${pad(schedule.minute)}`;
  }
  const { minutes } = schedule;
  if (minutes % 1440 === 0) return minutes === 1440 ? "Every day" : `Every ${minutes / 1440} days`;
  if (minutes % 60 === 0) return minutes === 60 ? "Every hour" : `Every ${minutes / 60} hours`;
  return `Every ${minutes} min`;
}

/**
 * Builds a schedule from form values: interval minutes as text, or a daily `HH:MM` time.
 * Returns an error message instead when the values are invalid.
 */
export function parseScheduleForm(
  kind: ScheduleKind,
  value: string
): { schedule: RoutineSchedule } | { error: string } {
  if (kind === "interval") {
    const minutes = Number(value);
    if (
      !Number.isInteger(minutes) ||
      minutes < minIntervalMinutes ||
      minutes > maxIntervalMinutes
    ) {
      return {
        error: `Interval must be a whole number from ${minIntervalMinutes} to ${maxIntervalMinutes} minutes.`
      };
    }
    return { schedule: { kind: "interval", minutes } };
  }
  const match = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(value);
  if (!match) return { error: "Daily time must be a valid HH:MM time." };
  return { schedule: { kind: "daily", hour: Number(match[1]), minute: Number(match[2]) } };
}

/** Formats an upcoming run as a short local date and time. */
export function formatNextRun(isoDate: string, now = new Date()): string {
  const date = new Date(isoDate);
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  const sameDay = date.toDateString() === now.toDateString();
  if (sameDay) return `Today ${time}`;
  const tomorrow = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  if (date.toDateString() === tomorrow.toDateString()) return `Tomorrow ${time}`;
  return `${date.toLocaleDateString("en-US", { month: "short", day: "numeric" })} ${time}`;
}

function pad(value: number): string {
  return String(value).padStart(2, "0");
}
