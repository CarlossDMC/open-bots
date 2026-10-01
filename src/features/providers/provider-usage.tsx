import { Gauge, Loader2, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ProviderUsageState } from "@/hooks/use-provider-usage";
import { cn, formatConversationTime } from "@/lib/utils";
import {
  formatResetShort,
  formatResetsIn,
  latestCheck,
  primaryWindows,
  providerShortName,
  summarizeUsage,
  windowLevel,
  windowName,
  windowShortLabel,
  type UsageLevel
} from "@/lib/usage";
import type { ProviderUsageReport } from "@/types/domain";
import { UsageMeter } from "./usage-meter";

const levelText: Record<UsageLevel, string> = {
  normal: "text-foreground-subtle",
  warning: "text-warning-foreground",
  danger: "text-danger-foreground"
};

/** Sidebar usage: one compact row per provider that opens the detailed breakdown. */
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
          "relative w-full rounded-md px-2 py-1.5 text-left transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          open && "bg-muted"
        )}
      >
        {usage.reports.length > 0 ? (
          <div className="space-y-1" aria-hidden="true">
            {usage.reports.map((report) => (
              <SidebarRow key={report.providerId} report={report} />
            ))}
          </div>
        ) : (
          <span className="flex items-center gap-2 text-2xs text-foreground-faint">
            <Gauge size={13} strokeWidth={1.8} aria-hidden="true" />
            {label}
          </span>
        )}
        {usage.loading && usage.reports.length > 0 && (
          <Loader2
            size={10}
            className="absolute right-1.5 top-1.5 animate-spin text-foreground-faint"
            aria-hidden="true"
          />
        )}
      </button>
      {open && (
        <div
          role="dialog"
          aria-label="Provider usage"
          className="absolute bottom-full left-3 z-40 mb-1 w-80 rounded-lg border border-border bg-card shadow-panel"
        >
          <div className="flex items-center justify-between border-b border-border-subtle px-3 py-2">
            <h2 className="text-xs font-medium text-foreground">Usage</h2>
            <div className="flex items-center gap-1.5">
              <CheckedAt usage={usage} />
              <RefreshButton usage={usage} />
            </div>
          </div>
          <div className="px-3 py-2.5">
            <ProviderUsageList usage={usage} />
          </div>
        </div>
      )}
    </div>
  );
}

function SidebarRow({ report }: { report: ProviderUsageReport }) {
  const windows = report.usage ? primaryWindows(report.usage) : [];
  return (
    <div className="grid grid-cols-[4.5rem_minmax(0,1fr)] items-center gap-2 text-2xs">
      <span className="truncate text-foreground-subtle" title={report.providerName}>
        {providerShortName(report.providerName)}
      </span>
      {report.usage ? (
        windows.length > 0 ? (
          <span className="flex min-w-0 items-center gap-3">
            {windows.map((window, index) => {
              const level = windowLevel(window, report.usage?.limitReached);
              return (
                <span
                  key={`${window.durationMinutes ?? "window"}-${index}`}
                  className="flex items-center gap-1.5 tabular-nums"
                >
                  <UsageMeter
                    size="sm"
                    percent={window.usedPercent}
                    level={level}
                    label={`${report.providerName} ${windowName(window.durationMinutes)}`}
                  />
                  <span className="text-foreground-faint">
                    {windowShortLabel(window.durationMinutes)}
                  </span>
                  <span className={levelText[level]}>{window.usedPercent}%</span>
                </span>
              );
            })}
          </span>
        ) : (
          <span className={report.usage.limitReached ? levelText.danger : "text-foreground-faint"}>
            {report.usage.limitReached ? "Limit reached" : "No limits"}
          </span>
        )
      ) : (
        <span className="truncate text-foreground-faint" title={report.error ?? undefined}>
          Unavailable
        </span>
      )}
    </div>
  );
}

function CheckedAt({ usage }: { usage: ProviderUsageState }) {
  const checked = latestCheck(usage.reports);
  if (!checked) return null;
  return (
    <span className="text-2xs text-foreground-faint">
      Checked {formatConversationTime(checked)}
    </span>
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

/** Per-provider usage tables with explicit loading, empty, and error states. */
export function ProviderUsageList({
  usage,
  showRefresh = false
}: {
  usage: ProviderUsageState;
  showRefresh?: boolean;
}) {
  const now = new Date();
  return (
    <div>
      {usage.error && (
        <p className="mb-2 text-xs text-danger-foreground" role="alert">
          {usage.error}
        </p>
      )}
      {usage.reports.length === 0 && !usage.error && (
        <p className="text-xs text-foreground-faint">
          {usage.loading ? "Reading usage…" : "No provider reports usage limits."}
        </p>
      )}
      <div className="divide-y divide-border-subtle">
        {usage.reports.map((report) => (
          <ProviderUsageTable key={report.providerId} report={report} now={now} />
        ))}
      </div>
      {showRefresh && (
        <div className="mt-2 flex items-center justify-end gap-1.5 border-t border-border-subtle pt-2">
          <CheckedAt usage={usage} />
          <RefreshButton usage={usage} />
        </div>
      )}
    </div>
  );
}

function ProviderUsageTable({ report, now }: { report: ProviderUsageReport; now: Date }) {
  const usage = report.usage;
  return (
    <section aria-label={`${report.providerName} usage`} className="py-2.5 first:pt-0 last:pb-0">
      <div className="mb-1.5 flex items-center gap-2">
        <h3 className="truncate text-xs text-foreground">{report.providerName}</h3>
        {usage?.plan && (
          <span className="shrink-0 rounded-sm border border-border-subtle px-1 text-3xs uppercase tracking-wider text-foreground-faint">
            {usage.plan.replaceAll("_", " ")}
          </span>
        )}
        {usage?.limitReached && (
          <span className="ml-auto shrink-0 rounded-sm border border-danger-border bg-danger-muted px-1 text-3xs uppercase tracking-wider text-danger-foreground">
            Limit reached
          </span>
        )}
      </div>
      {report.error && <p className="text-2xs text-danger-foreground">{report.error}</p>}
      {usage && usage.windows.length === 0 && (
        <p className="text-2xs text-foreground-faint">No limits reported.</p>
      )}
      {usage && usage.windows.length > 0 && (
        <div className="grid grid-cols-[minmax(0,6.5rem)_minmax(2rem,1fr)_2.5rem_3.5rem] items-center gap-x-2 gap-y-1.5 text-2xs">
          {usage.windows.map((window, index) => {
            const level = windowLevel(window, usage.limitReached);
            const name = windowName(window.durationMinutes, window.scope);
            return (
              <div key={`${name}-${index}`} className="contents">
                <span className="truncate text-foreground-subtle" title={name}>
                  {name}
                </span>
                <UsageMeter
                  percent={window.usedPercent}
                  level={level}
                  label={`${report.providerName} ${name}`}
                />
                <span className={cn("text-right tabular-nums", levelText[level])}>
                  {window.usedPercent}%
                </span>
                <span
                  className="text-right tabular-nums text-foreground-faint"
                  title={formatResetsIn(window.resetsAt, now)}
                >
                  {formatResetShort(window.resetsAt, now) ?? "—"}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
