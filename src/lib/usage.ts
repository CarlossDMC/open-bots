import type { ProviderUsage, ProviderUsageReport, UsageWindow } from "@/types/domain";

export type UsageLevel = "normal" | "warning" | "danger";

/** Usage at or above this percentage is highlighted before the limit is reached. */
export const usageWarningPercent = 80;

const minutesPerHour = 60;
const minutesPerDay = 24 * minutesPerHour;
const minutesPerWeek = 7 * minutesPerDay;

/** Compact window name such as "5h", "7d", or "7d Fable" for a scoped window. */
export function windowShortLabel(durationMinutes?: number | null, scope?: string | null): string {
  const label = durationShortLabel(durationMinutes);
  return scope ? `${label} ${scope}` : label;
}

/** Window name for a table row, such as "5-hour", "Weekly", or "Weekly · Fable". */
export function windowName(durationMinutes?: number | null, scope?: string | null): string {
  const label = durationName(durationMinutes);
  return scope ? `${label} · ${scope}` : label;
}

function durationShortLabel(durationMinutes?: number | null): string {
  if (!durationMinutes || durationMinutes <= 0) return "Limit";
  if (durationMinutes % minutesPerDay === 0) return `${durationMinutes / minutesPerDay}d`;
  if (durationMinutes >= minutesPerHour) return `${Math.round(durationMinutes / minutesPerHour)}h`;
  return `${durationMinutes}m`;
}

function durationName(durationMinutes?: number | null): string {
  if (durationMinutes === minutesPerWeek) return "Weekly";
  if (durationMinutes === minutesPerDay) return "Daily";
  if (durationMinutes && durationMinutes % minutesPerDay === 0) {
    return `${durationMinutes / minutesPerDay}-day`;
  }
  if (durationMinutes && durationMinutes >= minutesPerHour) {
    return `${Math.round(durationMinutes / minutesPerHour)}-hour`;
  }
  if (durationMinutes && durationMinutes > 0) return `${durationMinutes}-minute`;
  return "Limit";
}

/** Windows that cover all usage; scoped windows are shown only in the detailed view. */
export function primaryWindows(usage: ProviderUsage): UsageWindow[] {
  const unscoped = usage.windows.filter((window) => !window.scope);
  return unscoped.length > 0 ? unscoped : usage.windows;
}

/** Display name without a trailing "CLI", such as "OpenAI Codex" for "OpenAI Codex CLI". */
export function providerShortName(name: string): string {
  return name.replace(/\s+CLI$/i, "") || name;
}

interface Remaining {
  days: number;
  hours: number;
  minutes: number;
}

function remainingUntil(
  resetsAt: string | null | undefined,
  now: Date
): Remaining | "now" | undefined {
  if (!resetsAt) return undefined;
  const remainingMinutes = Math.ceil((new Date(resetsAt).getTime() - now.getTime()) / 60_000);
  if (Number.isNaN(remainingMinutes)) return undefined;
  if (remainingMinutes <= 0) return "now";
  return {
    days: Math.floor(remainingMinutes / minutesPerDay),
    hours: Math.floor((remainingMinutes % minutesPerDay) / minutesPerHour),
    minutes: remainingMinutes % minutesPerHour
  };
}

export function formatResetsIn(resetsAt: string | null | undefined, now: Date): string | undefined {
  const remaining = remainingUntil(resetsAt, now);
  if (!remaining) return undefined;
  if (remaining === "now") return "Resets now";
  const { days, hours, minutes } = remaining;
  if (days > 0) return `Resets in ${days}d ${hours}h`;
  if (hours > 0) return `Resets in ${hours}h ${minutes}m`;
  return `Resets in ${minutes}m`;
}

/** Fixed-width reset countdown for table columns, such as "2h 05m" or "3d 12h". */
export function formatResetShort(
  resetsAt: string | null | undefined,
  now: Date
): string | undefined {
  const remaining = remainingUntil(resetsAt, now);
  if (!remaining) return undefined;
  if (remaining === "now") return "now";
  const { days, hours, minutes } = remaining;
  const pad = (value: number) => String(value).padStart(2, "0");
  if (days > 0) return `${days}d ${pad(hours)}h`;
  if (hours > 0) return `${hours}h ${pad(minutes)}m`;
  return `${minutes}m`;
}

export function windowLevel(window: UsageWindow, limitReached = false): UsageLevel {
  if (limitReached || window.usedPercent >= 100) return "danger";
  return window.usedPercent >= usageWarningPercent ? "warning" : "normal";
}

export function usageLevel(usage: ProviderUsage): UsageLevel {
  if (usage.limitReached) return "danger";
  const levels = usage.windows.map((window) => windowLevel(window));
  if (levels.includes("danger")) return "danger";
  return levels.includes("warning") ? "warning" : "normal";
}

export interface UsageSummary {
  text: string;
  level: UsageLevel;
}

/** Accessible one-line summary, such as "Claude Code 5h 14%, 7d 20%; OpenAI Codex 5h 2%". */
export function summarizeUsage(reports: ProviderUsageReport[]): UsageSummary | undefined {
  const reported = reports.flatMap((report) =>
    report.usage ? [{ name: providerShortName(report.providerName), usage: report.usage }] : []
  );
  if (reported.length === 0) return undefined;
  const text = reported
    .map(({ name, usage }) => {
      const windows = primaryWindows(usage)
        .map((window) => `${windowShortLabel(window.durationMinutes)} ${window.usedPercent}%`)
        .join(", ");
      if (windows) return `${name} ${windows}`;
      return `${name} ${usage.limitReached ? "limit reached" : "no limits"}`;
    })
    .join("; ");
  const levels = reported.map(({ usage }) => usageLevel(usage));
  const level = levels.includes("danger")
    ? "danger"
    : levels.includes("warning")
      ? "warning"
      : "normal";
  return { text, level };
}

/** Most recent check time across reports, for a single "Checked" label. */
export function latestCheck(reports: ProviderUsageReport[]): string | undefined {
  return reports
    .flatMap((report) => (report.usage ? [report.usage.checkedAt] : []))
    .sort()
    .at(-1);
}
