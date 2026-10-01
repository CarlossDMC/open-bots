import { CircleX, ListTodo, Plus, RotateCcw, Search, SearchX } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { Input } from "@/components/ui/input";
import { StatusBadge } from "@/components/ui/status-badge";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { useTasks } from "@/hooks/use-tasks";
import { tasksUnavailableMessage } from "@/lib/desktop-api";
import { cn, formatRelativeTime, titleCase } from "@/lib/utils";
import { taskStatuses, type Agent, type AgentTask, type TaskStatus } from "@/types/domain";
import { NewTaskDialog } from "./new-task-dialog";

const allStatuses = "all";
const allAgents = "all";
const unassigned = "unassigned";
const finishedStatuses = new Set<TaskStatus>(["completed", "failed", "cancelled"]);

export function TasksPage({ agents }: { agents: Agent[] }) {
  const { tasks, available, loading, error, createError, create, clearCreateError, setStatus } =
    useTasks();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [status, setStatusFilter] = useState(allStatuses);
  const [assignee, setAssignee] = useState(allAgents);
  const agentsById = useMemo(() => new Map(agents.map((agent) => [agent.id, agent])), [agents]);

  const filteredTasks = useMemo(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase();
    return tasks.filter((task) => {
      const matchesQuery =
        normalizedQuery.length === 0 ||
        [task.title, task.description, task.result ?? ""].some((value) =>
          value.toLocaleLowerCase().includes(normalizedQuery)
        );
      const matchesStatus = status === allStatuses || task.status === status;
      const matchesAssignee =
        assignee === allAgents ||
        (assignee === unassigned ? !task.assignedAgentId : task.assignedAgentId === assignee);
      return matchesQuery && matchesStatus && matchesAssignee;
    });
  }, [assignee, query, status, tasks]);

  const filtersActive = query.length > 0 || status !== allStatuses || assignee !== allAgents;
  const openDialog = useCallback(() => {
    clearCreateError();
    setDialogOpen(true);
  }, [clearCreateError]);
  const closeDialog = useCallback(() => setDialogOpen(false), []);
  const clearFilters = () => {
    setQuery("");
    setStatusFilter(allStatuses);
    setAssignee(allAgents);
  };

  return (
    <div className="animate-fade-in">
      <header className="mb-7 flex items-start justify-between gap-6">
        <div>
          <h1 className="text-xl font-semibold tracking-tight text-foreground">Tasks</h1>
          <p className="mt-1 text-sm text-foreground-subtle">
            Track persistent work across your local agents.
          </p>
        </div>
        <Button className="shrink-0" onClick={openDialog} disabled={!available}>
          <Plus size={15} />
          New task
        </Button>
      </header>

      {!available ? (
        <p className="mb-4 rounded-md border border-border-subtle bg-muted/50 px-3 py-2 text-xs text-foreground-muted">
          {tasksUnavailableMessage}
        </p>
      ) : null}
      {error ? (
        <p role="alert" className="mb-4 text-xs text-danger-foreground">
          {error}
        </p>
      ) : null}

      <TaskFilters
        query={query}
        status={status}
        assignee={assignee}
        agents={agents}
        resultCount={filteredTasks.length}
        filtersActive={filtersActive}
        onQueryChange={setQuery}
        onStatusChange={setStatusFilter}
        onAssigneeChange={setAssignee}
        onClear={clearFilters}
      />

      {loading ? (
        <TaskListSkeleton />
      ) : tasks.length === 0 ? (
        <EmptyTasks available={available} onCreate={openDialog} />
      ) : filteredTasks.length === 0 ? (
        <NoMatches onClear={clearFilters} />
      ) : (
        <div className="overflow-hidden rounded-lg border border-border/80">
          <div className="hidden grid-cols-[minmax(15rem,1.8fr)_minmax(8rem,.65fr)_minmax(9rem,.8fr)_minmax(7rem,.65fr)_2rem] items-center gap-4 border-b border-border-subtle bg-card/70 px-4 py-2.5 text-2xs font-medium uppercase tracking-wider text-foreground-faint md:grid">
            <span>Task</span>
            <span>Status</span>
            <span>Assigned to</span>
            <span>Created</span>
            <span className="sr-only">Actions</span>
          </div>
          {filteredTasks.map((task) => (
            <TaskRow
              key={task.id}
              task={task}
              assignee={task.assignedAgentId ? agentsById.get(task.assignedAgentId) : undefined}
              creator={
                task.createdByAgentId
                  ? (agentsById.get(task.createdByAgentId)?.name ?? "Unknown agent")
                  : "You"
              }
              canCancel={available && !finishedStatuses.has(task.status)}
              onCancel={() => void setStatus(task, "cancelled")}
            />
          ))}
        </div>
      )}

      <NewTaskDialog
        open={dialogOpen}
        available={available}
        agents={agents}
        error={createError}
        onClose={closeDialog}
        onSubmit={create}
      />
    </div>
  );
}

function TaskFilters({
  query,
  status,
  assignee,
  agents,
  resultCount,
  filtersActive,
  onQueryChange,
  onStatusChange,
  onAssigneeChange,
  onClear
}: {
  query: string;
  status: string;
  assignee: string;
  agents: Agent[];
  resultCount: number;
  filtersActive: boolean;
  onQueryChange: (value: string) => void;
  onStatusChange: (value: string) => void;
  onAssigneeChange: (value: string) => void;
  onClear: () => void;
}) {
  const statusOptions: ComboboxOption[] = [
    { value: allStatuses, label: "All statuses" },
    ...taskStatuses.map((value) => ({ value, label: titleCase(value) }))
  ];
  const agentOptions: ComboboxOption[] = [
    { value: allAgents, label: "All agents" },
    { value: unassigned, label: "Unassigned" },
    ...agents.map((agent) => ({ value: agent.id, label: agent.name, description: agent.role }))
  ];

  return (
    <div className="mb-3 flex flex-col gap-2 lg:flex-row lg:items-center">
      <div className="relative min-w-0 flex-1 lg:max-w-sm">
        <Search
          size={14}
          className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-foreground-faint"
          aria-hidden="true"
        />
        <Input
          aria-label="Search tasks"
          type="search"
          className="pl-9"
          placeholder="Search tasks…"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
        />
      </div>
      <Combobox
        aria-label="Filter by status"
        className="lg:w-40"
        value={status}
        options={statusOptions}
        searchPlaceholder="Search statuses…"
        onChange={onStatusChange}
      />
      <Combobox
        aria-label="Filter by agent"
        className="lg:w-48"
        value={assignee}
        options={agentOptions}
        searchPlaceholder="Search agents…"
        onChange={onAssigneeChange}
      />
      <div className="flex min-h-9 items-center justify-between gap-2 lg:ml-auto lg:justify-end">
        <span className="whitespace-nowrap text-xs tabular-nums text-foreground-faint">
          {resultCount} {resultCount === 1 ? "task" : "tasks"}
        </span>
        {filtersActive ? (
          <Button variant="ghost" size="sm" onClick={onClear}>
            <RotateCcw size={13} />
            Clear
          </Button>
        ) : null}
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
  assignee?: Agent;
  creator: string;
  canCancel: boolean;
  onCancel: () => void;
}) {
  return (
    <article className="grid gap-3 border-b border-border-subtle bg-card/30 px-4 py-3.5 transition-colors last:border-0 hover:bg-muted/60 md:grid-cols-[minmax(15rem,1.8fr)_minmax(8rem,.65fr)_minmax(9rem,.8fr)_minmax(7rem,.65fr)_2rem] md:items-center md:gap-4">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium text-foreground">{task.title}</p>
        {task.result ? (
          <p className="mt-1 truncate text-xs text-foreground-muted">
            <span className="text-status-success">Result:</span> {task.result}
          </p>
        ) : task.description ? (
          <p className="mt-1 truncate text-xs text-foreground-faint">{task.description}</p>
        ) : (
          <p className="mt-1 text-xs text-foreground-faint">No description</p>
        )}
        <p className="mt-1 text-2xs text-foreground-faint md:hidden">Created by {creator}</p>
      </div>
      <StatusBadge status={task.status} />
      <div className="flex min-w-0 items-center gap-2">
        {assignee ? (
          <>
            <AgentAvatar
              color={assignee.identityColor}
              variant={assignee.avatarVariant}
              status={assignee.status}
              size="sm"
              seed={assignee.id}
            />
            <div className="min-w-0">
              <p className="truncate text-xs text-foreground-muted">{assignee.name}</p>
              <p className="truncate text-2xs text-foreground-faint">{assignee.role}</p>
            </div>
          </>
        ) : (
          <span className="text-xs text-foreground-faint">Unassigned</span>
        )}
      </div>
      <div className="min-w-0">
        <time
          dateTime={task.createdAt}
          title={new Date(task.createdAt).toLocaleString("en-US")}
          className="text-xs text-foreground-faint"
        >
          {formatRelativeTime(task.createdAt)}
        </time>
        <p className="hidden truncate text-2xs text-foreground-faint md:block">by {creator}</p>
      </div>
      <div className="flex justify-end md:block">
        {canCancel ? (
          <Button
            variant="ghost"
            size="icon"
            className="size-8 text-foreground-faint hover:text-danger-foreground"
            aria-label={`Cancel ${task.title}`}
            title="Cancel task"
            onClick={onCancel}
          >
            <CircleX size={14} />
          </Button>
        ) : null}
      </div>
    </article>
  );
}

function EmptyTasks({ available, onCreate }: { available: boolean; onCreate: () => void }) {
  return (
    <div className="grid min-h-72 place-items-center rounded-lg border border-dashed border-border text-center">
      <div className="max-w-sm px-6">
        <div className="mx-auto mb-3 grid size-10 place-items-center rounded-lg bg-muted text-foreground-faint">
          <ListTodo size={20} />
        </div>
        <p className="text-sm font-medium text-foreground-secondary">No tasks yet</p>
        <p className="mt-1 text-xs leading-5 text-foreground-faint">
          Create a task to track work independently or assign it to a local agent.
        </p>
        <Button size="sm" className="mt-4" onClick={onCreate} disabled={!available}>
          <Plus size={14} />
          Create task
        </Button>
      </div>
    </div>
  );
}

function NoMatches({ onClear }: { onClear: () => void }) {
  return (
    <div className="grid min-h-64 place-items-center rounded-lg border border-dashed border-border text-center">
      <div>
        <SearchX className="mx-auto mb-3 text-foreground-faint" size={24} />
        <p className="text-sm text-foreground-secondary">No matching tasks</p>
        <p className="mt-1 text-xs text-foreground-faint">
          Try another search or clear the filters.
        </p>
        <Button variant="ghost" size="sm" className="mt-3" onClick={onClear}>
          Clear filters
        </Button>
      </div>
    </div>
  );
}

function TaskListSkeleton() {
  return (
    <div
      role="status"
      aria-label="Loading tasks"
      className="overflow-hidden rounded-lg border border-border/80"
    >
      {[0, 1, 2].map((row) => (
        <div
          key={row}
          className="flex min-h-16 items-center gap-4 border-b border-border-subtle px-4 last:border-0"
        >
          <div className="h-3 w-1/3 rounded bg-muted" />
          <div className={cn("h-2.5 rounded bg-muted", row === 1 ? "w-24" : "w-16")} />
        </div>
      ))}
      <span className="sr-only">Loading tasks…</span>
    </div>
  );
}
