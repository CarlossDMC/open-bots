import {
  Box,
  Brain,
  CalendarClock,
  CheckCircle2,
  CircleDot,
  History,
  MessageSquare,
  ShieldCheck,
  Terminal,
  type LucideIcon
} from "lucide-react";
import type { ActivityEvent } from "@/types/domain";

export function ActivityPage({ events, error }: { events: ActivityEvent[]; error?: string }) {
  return (
    <div className="animate-fade-in">
      <header className="mb-7">
        <h1 className="text-xl font-semibold text-foreground">Activity</h1>
        <p className="mt-1 text-sm text-foreground-subtle">
          A persisted timeline of structured runtime events.
        </p>
      </header>
      {error ? (
        <p role="alert" className="mb-4 text-sm text-danger-foreground">
          {error}
        </p>
      ) : null}
      {events.length === 0 && !error ? (
        <div className="grid min-h-64 place-items-center rounded-lg border border-dashed border-border text-center">
          <div>
            <History className="mx-auto mb-3 text-foreground-faint" size={25} />
            <p className="text-sm text-foreground-secondary">No activity yet</p>
            <p className="mt-1 text-xs text-foreground-faint">
              Runtime events appear here as they are recorded.
            </p>
          </div>
        </div>
      ) : null}
      {events.length > 0 ? (
        <div className="relative ml-2 max-w-3xl before:absolute before:bottom-3 before:left-[11px] before:top-3 before:w-px before:bg-border">
          {events.map((event) => {
            const Icon = eventIcon(event.type);
            return (
              <div key={event.id} className="relative flex gap-4 py-3">
                <div className="z-10 grid size-6 shrink-0 place-items-center rounded-full border border-border bg-card">
                  <Icon size={11} className="text-foreground-subtle" />
                </div>
                <div className="flex min-w-0 flex-1 items-baseline justify-between gap-4">
                  <div>
                    <span className="text-sm font-medium text-foreground-secondary">
                      {event.subject}
                    </span>
                    <span className="ml-2 text-sm text-foreground-subtle">{event.detail}</span>
                    <p className="mt-1 font-mono text-2xs text-foreground-faint">{event.type}</p>
                  </div>
                  <time className="shrink-0 text-xs tabular-nums text-foreground-faint">
                    {new Date(event.occurredAt).toLocaleTimeString("en", {
                      hour: "2-digit",
                      minute: "2-digit"
                    })}
                  </time>
                </div>
              </div>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

function eventIcon(type: string): LucideIcon {
  if (type.startsWith("process")) return CheckCircle2;
  if (type.startsWith("tool")) return Terminal;
  if (type.startsWith("approval")) return ShieldCheck;
  if (type.startsWith("memory")) return Brain;
  if (type.startsWith("routine")) return CalendarClock;
  if (type.startsWith("artifact")) return Box;
  if (type.includes("message")) return MessageSquare;
  return CircleDot;
}
