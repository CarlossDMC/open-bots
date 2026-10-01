import type { ProviderUsage, ProviderUsageReport, UsageWindow } from "@/types/domain";

export type UsageLevel = "normal" | "warning" | "danger";

/** Usage at or above this percentage is highlighted before the limit is reached. */
export const usageWarningPercent = 80;

const minutesPerHour = 60;
const minutesPerDay = 24 * minutesPerHour;
const minutesPerWeek = 7 * minutesPerDay;

/** Compact window name such as "5h" or "7d". */
export function windowShortLabel(durationMinutes?: number | null): string {
  if (!durationMinutes || durationMinutes <= 0) return "Limit";
  if (durationMinutes % minutesPerDay === 0) return `${durationMinutes / minutesPerDay}d`;
  if (durationMinutes >= minutesPerHour) return `${Math.round(durationMinutes / minutesPerHour)}h`;
  return `${durationMinutes}m`;
}

/** Readable window name such as "5-hour limit" or "Weekly limit". */
export function windowLongLabel(durationMinutes?: number | null): string {
  if (durationMinutes === minutesPerWeek) return "Weekly limit";
  if (durationMinutes === minutesPerDay) return "Daily limit";
  if (durationMinutes && durationMinutes % minutesPerDay === 0) {
    return `${durationMinutes / minutesPerDay}-day limit`;
  }
  if (durationMinutes && durationMinutes >= minutesPerHour) {
    return `${Math.round(durationMinutes / minutesPerHour)}-hour limit`;
  }
  if (durationMinutes && durationMinutes > 0) return `${durationMinutes}-minute limit`;
  return "Usage limit";
}

export function formatResetsIn(resetsAt: string | null | undefined, now: Date): string | undefined {
  if (!resetsAt) return undefined;
  const remainingMinutes = Math.ceil((new Date(resetsAt).getTime() - now.getTime()) / 60_000);
  if (Number.isNaN(remainingMinutes)) return undefined;
  if (remainingMinutes <= 0) return "Resets now";
  const days = Math.floor(remainingMinutes / minutesPerDay);
  const hours = Math.floor((remainingMinutes % minutesPerDay) / minutesPerHour);
  const minutes = remainingMinutes % minutesPerHour;
  if (days > 0) return `Resets in ${days}d ${hours}h`;
  if (hours > 0) return `Resets in ${hours}h ${minutes}m`;
  return `Resets in ${minutes}m`;
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

/** One-line summary of every reported window, such as "5h 2% · 7d 30%". */
export function summarizeUsage(reports: ProviderUsageReport[]): UsageSummary | undefined {
  const usages = reports.flatMap((report) => (report.usage ? [report.usage] : []));
  if (usages.length === 0) return undefined;
  const text = usages
    .flatMap((usage) =>
      usage.windows.map(
        (window) => `${windowShortLabel(window.durationMinutes)} ${window.usedPercent}%`
      )
    )
    .join(" · ");
  const levels = usages.map(usageLevel);
  const level = levels.includes("danger")
    ? "danger"
    : levels.includes("warning")
      ? "warning"
      : "normal";
  return { text: text || (level === "danger" ? "Limit reached" : "No limits"), level };
}
