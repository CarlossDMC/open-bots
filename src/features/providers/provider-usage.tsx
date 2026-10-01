import { Gauge, Loader2, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ProviderUsageState } from "@/hooks/use-provider-usage";
import { cn, formatConversationTime } from "@/lib/utils";
import {
  formatResetsIn,
  summarizeUsage,
  windowLevel,
  windowLongLabel,
  type UsageLevel
} from "@/lib/usage";
import type { ProviderUsageReport, UsageWindow } from "@/types/domain";

const levelText: Record<UsageLevel, string> = {
  normal: "text-foreground-subtle",
  warning: "text-warning-foreground",
  danger: "text-danger-foreground"
};

const levelBar: Record<UsageLevel, string> = {
  normal: "bg-foreground-subtle",
  warning: "bg-warning",
  danger: "bg-danger"
};

/** Compact usage summary for the sidebar that opens the per-provider breakdown. */
export function UsageIndicator({ usage }: { usage: ProviderUsageState }) {
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const summary = summarizeUsage(usage.reports);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setOpen(false);
      trigger.current?.focus();
    };
    const onPointer = (event: PointerEvent) => {
      if (!container.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointer);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointer);
    };
  }, [open]);

  let label: string;
  if (summary) label = summary.text;
  else if (usage.loading) label = "Reading usage…";
  else if (usage.error || usage.reports.some((report) => report.error)) label = "Usage unavailable";
  else label = "No usage reported";

  return (
    <div ref={container} className="relative px-3 pb-1.5">
      <button
        ref={trigger}
        type="button"
        onClick={() => {
          if (!open) void usage.refresh();
          setOpen((current) => !current);
        }}
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-label={`Provider usage: ${label}`}
        className={cn(
          "flex h-7 w-full items-center gap-2 rounded-md px-2 text-2xs tabular-nums transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          levelText[summary?.level ?? "normal"]
        )}
      >
        <Gauge size={13} strokeWidth={1.8} aria-hidden="true" />
        <span className="truncate">{label}</span>
        {usage.loading && (
          <Loader2 size={11} className="ml-auto animate-spin text-foreground-faint" aria-hidden />
        )}
      </button>
      {open && (
        <div
          role="dialog"
          aria-label="Provider usage"
          className="absolute bottom-full left-3 right-3 z-40 mb-1 rounded-lg border border-border bg-card p-3 shadow-panel"
        >
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-xs font-medium text-foreground">Provider usage</h2>
            <RefreshButton usage={usage} />
          </div>
          <ProviderUsageList usage={usage} />
        </div>
      )}
    </div>
  );
}

function RefreshButton({ usage }: { usage: ProviderUsageState }) {
  return (
    <button
      type="button"
      onClick={() => void usage.refresh()}
      disabled={usage.loading}
      aria-label="Refresh usage"
      title="Refresh usage"
      className="grid size-6 place-items-center rounded-md text-foreground-subtle transition-colors hover:bg-muted hover:text-foreground disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    >
      <RefreshCw size={12} className={cn(usage.loading && "animate-spin")} aria-hidden="true" />
    </button>
  );
}

/** Per-provider usage windows with explicit loading, empty, and error states. */
export function ProviderUsageList({
  usage,
  showRefresh = false
}: {
  usage: ProviderUsageState;
  showRefresh?: boolean;
}) {
  const now = new Date();
  return (
    <div className="space-y-3">
      {showRefresh && (
        <div className="flex justify-end">
          <RefreshButton usage={usage} />
        </div>
      )}
      {usage.error && (
        <p className="text-xs text-danger-foreground" role="alert">
          {usage.error}
        </p>
      )}
      {usage.reports.length === 0 && !usage.error && (
        <p className="text-xs text-foreground-faint">
          {usage.loading ? "Reading usage…" : "No provider reports usage limits."}
        </p>
      )}
      {usage.reports.map((report) => (
        <ProviderUsageBlock key={report.providerId} report={report} now={now} />
      ))}
    </div>
  );
}

function ProviderUsageBlock({ report, now }: { report: ProviderUsageReport; now: Date }) {
  const usage = report.usage;
  return (
    <section aria-label={`${report.providerName} usage`}>
      <div className="flex items-baseline justify-between gap-2">
        <p className="truncate text-xs text-foreground">{report.providerName}</p>
        {usage?.plan && (
          <span className="shrink-0 text-2xs uppercase tracking-wider text-foreground-faint">
            {usage.plan.replaceAll("_", " ")}
          </span>
        )}
      </div>
      {report.error && <p className="mt-1 text-xs text-danger-foreground">{report.error}</p>}
      {usage && (
        <>
          {usage.limitReached && (
            <p className="mt-1 text-xs text-danger-foreground">Usage limit reached.</p>
          )}
          <div className="mt-2 space-y-2.5">
            {usage.windows.map((limit, index) => (
              <UsageBar
                key={`${limit.durationMinutes ?? "window"}-${index}`}
                limit={limit}
                limitReached={usage.limitReached}
                now={now}
              />
            ))}
          </div>
          <p className="mt-2 text-2xs text-foreground-faint">
            Checked {formatConversationTime(usage.checkedAt)}
          </p>
        </>
      )}
    </section>
  );
}

function UsageBar({
  limit,
  limitReached,
  now
}: {
  limit: UsageWindow;
  limitReached: boolean;
  now: Date;
}) {
  const level = windowLevel(limit, limitReached);
  const label = windowLongLabel(limit.durationMinutes);
  const resets = formatResetsIn(limit.resetsAt, now);
  const percent = Math.min(100, Math.max(0, limit.usedPercent));
  return (
    <div>
      <div className="flex items-baseline justify-between text-2xs">
        <span className="text-foreground-subtle">{label}</span>
        <span className={cn("tabular-nums", levelText[level])}>{limit.usedPercent}% used</span>
      </div>
      <div
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        className="mt-1 h-1 overflow-hidden rounded-full bg-muted"
      >
        <div
          className={cn("h-full rounded-full", levelBar[level])}
          style={{ width: `${percent}%` }}
        />
      </div>
      {resets && <p className="mt-1 text-2xs text-foreground-faint">{resets}</p>}
    </div>
  );
}
