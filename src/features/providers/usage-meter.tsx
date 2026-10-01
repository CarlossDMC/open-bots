import { cn } from "@/lib/utils";
import type { UsageLevel } from "@/lib/usage";

const levelFill: Record<UsageLevel, string> = {
  normal: "bg-foreground-subtle",
  warning: "bg-warning",
  danger: "bg-danger"
};

/** Thin usage bar colored by level; `sm` fits sidebar rows, `md` fills a table column. */
export function UsageMeter({
  percent,
  level,
  label,
  size = "md"
}: {
  percent: number;
  level: UsageLevel;
  label: string;
  size?: "sm" | "md";
}) {
  const value = Math.min(100, Math.max(0, Math.round(percent)));
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value}
      className={cn(
        "shrink-0 overflow-hidden rounded-full bg-muted",
        size === "sm" ? "h-1 w-6" : "h-1.5 w-full min-w-8"
      )}
    >
      <div
        className={cn(
          "h-full origin-left rounded-full motion-safe:transition-transform motion-safe:duration-base motion-safe:ease-standard",
          levelFill[level]
        )}
        style={{ transform: `scaleX(${value / 100})` }}
      />
    </div>
  );
}
