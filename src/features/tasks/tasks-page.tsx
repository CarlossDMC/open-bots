import { StatusBadge } from "@/components/ui/status-badge";
import { formatRelativeTime } from "@/lib/utils";
import type { Agent, AgentTask } from "@/types/domain";

export function TasksPage({ tasks, agents }: { tasks: AgentTask[]; agents: Agent[] }) {
  const names = new Map(agents.map((agent) => [agent.id, agent.name]));
  return (
    <div className="animate-fade-in">
      <PageHeading title="Tasks" description="Persistent units of work owned by agents." />
      <div className="table-shell">
        <div className="table-header grid-cols-[1.7fr_.7fr_.8fr_.5fr]">
          <span>Task</span>
          <span>Status</span>
          <span>Assigned agent</span>
          <span>Created</span>
        </div>
        {tasks.map((task) => (
          <div key={task.id} className="table-row grid-cols-[1.7fr_.7fr_.8fr_.5fr]">
            <div>
              <p className="text-sm text-foreground">{task.title}</p>
              <p className="mt-0.5 truncate text-xs text-foreground-faint">{task.description}</p>
            </div>
            <StatusBadge status={task.status} />
            <span className="text-xs text-foreground-muted">
              {task.assignedAgentId
                ? (names.get(task.assignedAgentId) ?? "Unknown agent")
                : "Unassigned"}
            </span>
            <span className="text-xs text-foreground-faint">
              {formatRelativeTime(task.createdAt, new Date("2026-01-10T11:00:00Z"))}
            </span>
          </div>
        ))}
      </div>
    </div>
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
