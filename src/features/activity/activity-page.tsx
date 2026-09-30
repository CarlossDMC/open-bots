import { Box, CheckCircle2, CircleDot, MessageSquare, Terminal } from "lucide-react";
import type { ActivityEvent } from "@/types/domain";

export function ActivityPage({ events }: { events: ActivityEvent[] }) {
  return (
    <div className="animate-fade-in">
      <header className="mb-7">
        <h1 className="text-xl font-semibold text-zinc-100">Activity</h1>
        <p className="mt-1 text-sm text-zinc-500">
          A persisted timeline of structured runtime events.
        </p>
      </header>
      <div className="relative ml-2 max-w-3xl before:absolute before:bottom-3 before:left-[11px] before:top-3 before:w-px before:bg-zinc-800">
        {events.map((event) => {
          const Icon = event.type.includes("process")
            ? CheckCircle2
            : event.type.includes("tool")
              ? Terminal
              : event.type.includes("artifact")
                ? Box
                : event.type.includes("message")
                  ? MessageSquare
                  : CircleDot;
          return (
            <div key={event.id} className="relative flex gap-4 py-3">
              <div className="z-10 grid size-6 shrink-0 place-items-center rounded-full border border-zinc-800 bg-zinc-950">
                <Icon size={11} className="text-zinc-500" />
              </div>
              <div className="flex min-w-0 flex-1 items-baseline justify-between gap-4">
                <div>
                  <span className="text-sm font-medium text-zinc-300">{event.subject}</span>
                  <span className="ml-2 text-sm text-zinc-500">{event.detail}</span>
                  <p className="mt-1 font-mono text-[10px] text-zinc-700">{event.type}</p>
                </div>
                <time className="shrink-0 text-xs tabular-nums text-zinc-600">
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
    </div>
  );
}
