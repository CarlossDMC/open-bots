import { Loader2, X } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { Input } from "@/components/ui/input";
import { StatusBadge } from "@/components/ui/status-badge";
import { useTasks } from "@/hooks/use-tasks";
import { tasksUnavailableMessage } from "@/lib/desktop-api";
import { formatRelativeTime } from "@/lib/utils";
import type { Agent, AgentTask } from "@/types/domain";

const unassigned = "";
const finishedStatuses = new Set(["completed", "failed", "cancelled"]);

export function TasksPage({ agents }: { agents: Agent[] }) {
  const { tasks, available, loading, error, create, setStatus } = useTasks();
  const names = new Map(agents.map((agent) => [agent.id, agent.name]));
  const agentName = (id?: string | null) => (id ? (names.get(id) ?? "Unknown agent") : undefined);

  return (
    <div className="animate-fade-in">
      <PageHeading title="Tasks" description="Persistent units of work owned by agents." />
      <NewTaskForm agents={agents} available={available} onCreate={create} />
      {error ? (
        <p role="alert" className="mb-3 text-xs text-danger-foreground">
          {error}
        </p>
      ) : null}
      <div className="table-shell">
        <div className="table-header grid-cols-[1.7fr_.6fr_.7fr_.7fr_.5fr_1.5rem]">
          <span>Task</span>
          <span>Status</span>
          <span>Assigned agent</span>
          <span>Created by</span>
          <span>Created</span>
          <span className="sr-only">Actions</span>
        </div>
        {loading ? (
          <p className="px-4 py-3 text-xs text-foreground-faint">Loading tasks…</p>
        ) : tasks.length === 0 ? (
          <p className="px-4 py-3 text-xs text-foreground-faint">No tasks yet.</p>
        ) : (
          tasks.map((task) => (
            <TaskRow
              key={task.id}
              task={task}
              assignee={agentName(task.assignedAgentId) ?? "Unassigned"}
              creator={agentName(task.createdByAgentId) ?? "You"}
              canCancel={available && !finishedStatuses.has(task.status)}
              onCancel={() => void setStatus(task, "cancelled")}
            />
          ))
        )}
      </div>
    </div>
  );
}

function TaskRow({
  task,
  assignee,
  creator,
  canCancel,
  onCancel
}: {
  task: AgentTask;
  assignee: string;
  creator: string;
  canCancel: boolean;
  onCancel: () => void;
}) {
  return (
    <div className="table-row grid-cols-[1.7fr_.6fr_.7fr_.7fr_.5fr_1.5rem]">
      <div className="min-w-0">
        <p className="truncate text-sm text-foreground">{task.title}</p>
        {task.result ? (
          <p className="mt-0.5 truncate text-xs text-foreground-muted">Result: {task.result}</p>
        ) : task.description ? (
          <p className="mt-0.5 truncate text-xs text-foreground-faint">{task.description}</p>
        ) : null}
      </div>
      <StatusBadge status={task.status} />
      <span className="truncate text-xs text-foreground-muted">{assignee}</span>
      <span className="truncate text-xs text-foreground-muted">{creator}</span>
      <span className="text-xs text-foreground-faint">{formatRelativeTime(task.createdAt)}</span>
      {canCancel ? (
        <Button
          variant="ghost"
          size="icon"
          className="size-6"
          aria-label={`Cancel ${task.title}`}
          onClick={onCancel}
        >
          <X size={12} />
        </Button>
      ) : (
        <span />
      )}
    </div>
  );
}

function NewTaskForm({
  agents,
  available,
  onCreate
}: {
  agents: Agent[];
  available: boolean;
  onCreate: ReturnType<typeof useTasks>["create"];
}) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [assignee, setAssignee] = useState(unassigned);
  const [saving, setSaving] = useState(false);
  const options: ComboboxOption[] = [
    { value: unassigned, label: "Unassigned" },
    ...agents.map((agent) => ({ value: agent.id, label: agent.name, description: agent.role }))
  ];
  const canSubmit = available && !saving && title.trim().length > 0;

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    setSaving(true);
    const created = await onCreate({
      title,
      description,
      assignedAgentId: assignee === unassigned ? undefined : assignee
    });
    if (created) {
      setTitle("");
      setDescription("");
    }
    setSaving(false);
  }

  return (
    <form className="mb-4 grid gap-2" onSubmit={(event) => void handleSubmit(event)}>
      <div className="flex gap-2">
        <Input
          aria-label="Task title"
          placeholder="New task"
          className="w-72 shrink-0"
          maxLength={200}
          value={title}
          disabled={!available}
          onChange={(event) => setTitle(event.target.value)}
        />
        <Input
          aria-label="Task description"
          placeholder="Details (optional)"
          maxLength={4000}
          value={description}
          disabled={!available}
          onChange={(event) => setDescription(event.target.value)}
        />
        <Combobox
          aria-label="Assigned agent"
          className="w-48 shrink-0"
          value={assignee}
          disabled={!available}
          searchPlaceholder="Search agents…"
          options={options}
          onChange={setAssignee}
        />
        <Button type="submit" size="sm" className="h-9 shrink-0" disabled={!canSubmit}>
          {saving ? <Loader2 size={13} className="animate-spin" /> : null}
          Add task
        </Button>
      </div>
      {available ? null : (
        <p className="text-xs text-foreground-faint">{tasksUnavailableMessage}</p>
      )}
    </form>
  );
}

function PageHeading({ title, description }: { title: string; description: string }) {
  return (
    <header className="mb-7">
      <h1 className="text-xl font-semibold tracking-tight text-foreground">{title}</h1>
      <p className="mt-1 text-sm text-foreground-subtle">{description}</p>
    </header>
  );
}
