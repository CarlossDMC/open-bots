import { cn, titleCase } from "@/lib/utils";

const statusStyles: Record<string, string> = {
  working: "bg-blue-400",
  running: "bg-blue-400",
  waiting: "bg-amber-400",
  paused: "bg-zinc-500",
  pending: "bg-zinc-500",
  queued: "bg-violet-400",
  failed: "bg-red-400",
  blocked: "bg-red-400",
  completed: "bg-emerald-400",
  approved: "bg-emerald-400",
  denied: "bg-red-400",
  cancelled: "bg-zinc-600",
  idle: "bg-zinc-500"
};

export function StatusBadge({ status, className }: { status: string; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-1.5 text-xs text-zinc-400", className)}>
      <span className={cn("size-1.5 rounded-full", statusStyles[status] ?? "bg-zinc-500")} />
      {titleCase(status)}
    </span>
  );
}
