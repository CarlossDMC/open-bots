import { cn, titleCase } from "@/lib/utils";

const statusStyles: Record<string, string> = {
  working: "bg-status-running",
  running: "bg-status-running",
  waiting: "bg-status-waiting",
  paused: "bg-status-neutral",
  pending: "bg-status-neutral",
  queued: "bg-status-queued",
  failed: "bg-status-danger",
  blocked: "bg-status-danger",
  completed: "bg-status-success",
  approved: "bg-status-success",
  denied: "bg-status-danger",
  cancelled: "bg-status-neutral/70",
  idle: "bg-status-neutral"
};

export function StatusBadge({ status, className }: { status: string; className?: string }) {
  return (
    <span
      className={cn("inline-flex items-center gap-1.5 text-xs text-foreground-muted", className)}
    >
      <span className={cn("size-1.5 rounded-full", statusStyles[status] ?? "bg-status-neutral")} />
      {titleCase(status)}
    </span>
  );
}
