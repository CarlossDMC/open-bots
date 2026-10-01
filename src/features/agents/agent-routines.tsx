import { CalendarClock, Loader2, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { Input } from "@/components/ui/input";
import { useAgentRoutines } from "@/hooks/use-agent-routines";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { routinesUnavailableMessage } from "@/lib/desktop-api";
import {
  describeSchedule,
  formatNextRun,
  maxRoutinesPerAgent,
  minIntervalMinutes,
  parseScheduleForm,
  type ScheduleKind
} from "@/lib/routines";
import { formatRelativeTime } from "@/lib/utils";

const scheduleOptions: ComboboxOption[] = [
  { value: "daily", label: "Daily at" },
  { value: "interval", label: "Every (minutes)" }
];

export function AgentRoutinesSection({ agentId }: { agentId: string }) {
  const { routines, available, loading, error, create, setEnabled, remove, reload } =
    useAgentRoutines(agentId);
  const [name, setName] = useState("");
  const [instructions, setInstructions] = useState("");
  const [kind, setKind] = useState<ScheduleKind>("daily");
  const [intervalMinutes, setIntervalMinutes] = useState("60");
  const [time, setTime] = useState("09:00");
  const [formError, setFormError] = useState<string>();
  const [saving, setSaving] = useState(false);

  // Keep last and next run times current when the scheduler fires one of these routines.
  useRuntimeEvents((event) => {
    if (event.eventType === "routine.triggered" && event.payload.agentId === agentId) {
      void reload();
    }
  });

  const atLimit = routines.length >= maxRoutinesPerAgent;
  const canSubmit = available && !atLimit && !saving && name.trim() && instructions.trim();

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    const parsed = parseScheduleForm(kind, kind === "interval" ? intervalMinutes : time);
    if ("error" in parsed) {
      setFormError(parsed.error);
      return;
    }
    setFormError(undefined);
    setSaving(true);
    if (await create({ agentId, name, instructions, schedule: parsed.schedule })) {
      setName("");
      setInstructions("");
    }
    setSaving(false);
  }

  return (
    <section className="panel lg:col-span-5" aria-labelledby="agent-routines-title">
      <h2 id="agent-routines-title" className="section-title">
        <CalendarClock size={14} /> Routines
      </h2>
      <p className="mb-3 text-xs text-foreground-faint">
        {available
          ? "When a routine is due, the agent runs its instructions as a turn while Open Bots is open. Runs missed while it was closed collapse into one."
          : routinesUnavailableMessage}
      </p>
      <form className="grid gap-2" onSubmit={(event) => void handleSubmit(event)}>
        <div className="flex gap-2">
          <Input
            aria-label="Routine name"
            placeholder="Name"
            className="w-48 shrink-0"
            maxLength={80}
            value={name}
            disabled={!available}
            onChange={(event) => setName(event.target.value)}
          />
          <Input
            aria-label="Routine instructions"
            placeholder="What should the agent do?"
            maxLength={4000}
            value={instructions}
            disabled={!available}
            onChange={(event) => setInstructions(event.target.value)}
          />
        </div>
        <div className="flex items-center gap-2">
          <Combobox
            aria-label="Schedule type"
            className="w-44 shrink-0"
            value={kind}
            disabled={!available}
            searchPlaceholder="Search schedules…"
            options={scheduleOptions}
            onChange={(value) => setKind(value as ScheduleKind)}
          />
          {kind === "daily" ? (
            <Input
              aria-label="Daily time"
              type="time"
              className="w-32"
              value={time}
              disabled={!available}
              onChange={(event) => setTime(event.target.value)}
            />
          ) : (
            <Input
              aria-label="Interval in minutes"
              type="number"
              className="w-32"
              min={minIntervalMinutes}
              value={intervalMinutes}
              disabled={!available}
              onChange={(event) => setIntervalMinutes(event.target.value)}
            />
          )}
          <Button type="submit" size="sm" className="h-9" disabled={!canSubmit}>
            {saving ? <Loader2 size={13} className="animate-spin" /> : null}
            Add routine
          </Button>
          {atLimit ? (
            <span className="text-xs text-foreground-faint">
              Limit of {maxRoutinesPerAgent} reached
            </span>
          ) : null}
        </div>
      </form>
      {formError || error ? (
        <p role="alert" className="mt-2 text-xs text-danger-foreground">
          {formError ?? error}
        </p>
      ) : null}
      {loading ? (
        <p className="mt-4 text-xs text-foreground-faint">Loading routines…</p>
      ) : routines.length === 0 ? (
        available ? (
          <p className="mt-4 text-xs text-foreground-faint">No routines yet.</p>
        ) : null
      ) : (
        <ul className="mt-3 divide-y divide-border-subtle">
          {routines.map((routine) => (
            <li key={routine.id} className="flex items-center gap-3 py-2">
              <Checkbox
                aria-label={`Enable ${routine.name}`}
                checked={routine.enabled}
                onCheckedChange={(checked) => void setEnabled(routine, checked)}
              />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm text-foreground-secondary">
                  {routine.name}
                  <span className="ml-2 text-xs text-foreground-faint">
                    {describeSchedule(routine.schedule)}
                  </span>
                </p>
                <p className="truncate text-xs text-foreground-faint">{routine.instructions}</p>
              </div>
              <div className="shrink-0 text-right text-2xs tabular-nums text-foreground-faint">
                <p>{routine.enabled ? `Next ${formatNextRun(routine.nextRunAt)}` : "Paused"}</p>
                <p>
                  {routine.lastRunAt
                    ? `Last ${formatRelativeTime(routine.lastRunAt)}`
                    : "Never run"}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="size-6 shrink-0"
                aria-label={`Delete ${routine.name}`}
                onClick={() => void remove(routine)}
              >
                <Trash2 size={12} />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
