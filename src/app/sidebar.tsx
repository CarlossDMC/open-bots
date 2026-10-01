import { Activity, CheckSquare2, ListTodo, Plus, Search, Settings, Users } from "lucide-react";
import { useMemo, useState } from "react";
import { ThemeToggle } from "@/features/appearance/theme-toggle";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { UsageIndicator } from "@/features/providers/provider-usage";
import type { ProviderUsageState } from "@/hooks/use-provider-usage";
import { cn, formatConversationTime } from "@/lib/utils";
import type { Agent, Group } from "@/types/domain";

export type ViewId = "chat" | "tasks" | "activity" | "approvals" | "settings";

const footerNavigation = [
  { id: "tasks", label: "Tasks", icon: ListTodo },
  { id: "activity", label: "Activity", icon: Activity },
  { id: "approvals", label: "Approvals", icon: CheckSquare2 },
  { id: "settings", label: "Settings", icon: Settings }
] satisfies { id: ViewId; label: string; icon: typeof ListTodo }[];

interface SidebarProps {
  active: ViewId;
  agents: Agent[];
  groups: Group[];
  selectedAgentId?: string;
  selectedGroupId?: string;
  isDemo: boolean;
  loading: boolean;
  error?: string;
  onNavigate: (view: ViewId) => void;
  onSelectAgent: (agent: Agent) => void;
  onSelectGroup: (group: Group) => void;
  onCreateAgent: () => void;
  onCreateGroup: () => void;
  onRetry: () => void;
  usage: ProviderUsageState;
}

export function Sidebar({
  active,
  agents,
  groups,
  selectedAgentId,
  selectedGroupId,
  isDemo,
  loading,
  error,
  onNavigate,
  onSelectAgent,
  onSelectGroup,
  onCreateAgent,
  onCreateGroup,
  onRetry,
  usage
}: SidebarProps) {
  const [query, setQuery] = useState("");
  const visibleAgents = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return agents;
    return agents.filter((agent) =>
      `${agent.name} ${agent.role}`.toLowerCase().includes(normalized)
    );
  }, [agents, query]);
  const visibleGroups = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return groups;
    return groups.filter((group) =>
      `${group.name} ${group.topic}`.toLowerCase().includes(normalized)
    );
  }, [groups, query]);

  return (
    <aside className="flex w-72 shrink-0 flex-col border-r border-border-subtle bg-surface">
      <div className="flex h-12 items-center pl-4 pr-2">
        <span className="text-sm font-semibold text-foreground">Open Bots</span>
      </div>
      <div className="px-3 pb-2">
        <label className="relative block">
          <span className="sr-only">Search agents and groups</span>
          <Search
            size={15}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-foreground-faint"
            aria-hidden="true"
          />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search"
            className="h-9 w-full rounded-md border border-border bg-card pl-8 pr-3 text-sm text-foreground outline-none placeholder:text-foreground-faint focus:border-border-strong focus:ring-1 focus:ring-ring/40"
          />
        </label>
        {isDemo && (
          <p className="mt-2 rounded-md border border-warning-border bg-warning-muted px-2 py-1.5 text-xs-plus text-warning-foreground">
            Browser preview uses demonstration data.
          </p>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <nav className="space-y-0.5 px-3 pb-2" aria-label="Groups">
          <SectionHeading label="Groups" actionLabel="New group" onAction={onCreateGroup} />
          {groups.length === 0 ? (
            <p className="px-2 pb-1 text-xs text-foreground-faint">No groups yet.</p>
          ) : visibleGroups.length === 0 ? (
            <p className="px-2 pb-1 text-xs text-foreground-faint">No groups match “{query}”.</p>
          ) : (
            visibleGroups.map((group) => {
              const selected = active === "chat" && group.id === selectedGroupId;
              const speaker = agents.find((agent) => agent.id === group.round.queue[0]?.agentId);
              return (
                <button
                  key={group.id}
                  type="button"
                  onClick={() => onSelectGroup(group)}
                  aria-current={selected ? "page" : undefined}
                  className={cn(
                    "flex h-[3.75rem] w-full items-center gap-3 rounded-lg px-2.5 text-left transition-colors",
                    selected ? "bg-accent" : "hover:bg-muted"
                  )}
                >
                  <span className="grid size-9 shrink-0 place-items-center rounded-lg border border-border bg-card text-foreground-muted">
                    <Users size={15} strokeWidth={1.8} aria-hidden="true" />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="truncate text-sm font-medium text-foreground">
                        {group.name}
                      </span>
                      <span className="shrink-0 text-xs-plus text-foreground-faint">
                        {formatConversationTime(group.updatedAt)}
                      </span>
                    </div>
                    <p className="mt-1 truncate text-xs text-foreground-subtle">
                      {speaker ? `${speaker.name} is answering` : group.topic}
                    </p>
                  </div>
                </button>
              );
            })
          )}
        </nav>
        <nav className="space-y-0.5 px-3" aria-label="Agents">
          <SectionHeading label="Agents" actionLabel="New agent" onAction={onCreateAgent} />
          {loading ? (
            <p className="px-2 py-3 text-xs text-foreground-faint">Loading local state…</p>
          ) : error ? (
            <div className="rounded-md border border-danger-border bg-danger-muted p-3 text-xs text-danger-foreground">
              <p>{error}</p>
              <button className="mt-2 underline" onClick={onRetry}>
                Try again
              </button>
            </div>
          ) : agents.length === 0 ? (
            <p className="px-2 py-3 text-xs text-foreground-faint">No agents yet.</p>
          ) : visibleAgents.length === 0 ? (
            <p className="px-2 py-3 text-xs text-foreground-faint">No agents match “{query}”.</p>
          ) : (
            visibleAgents.map((agent) => (
              <button
                key={agent.id}
                type="button"
                onClick={() => onSelectAgent(agent)}
                aria-current={
                  active === "chat" && agent.id === selectedAgentId ? "page" : undefined
                }
                className={cn(
                  "flex h-[3.75rem] w-full items-center gap-3 rounded-lg px-2.5 text-left transition-colors",
                  active === "chat" && agent.id === selectedAgentId ? "bg-accent" : "hover:bg-muted"
                )}
              >
                <AgentAvatar
                  color={agent.identityColor}
                  variant={agent.avatarVariant}
                  seed={agent.id}
                  status={agent.status}
                  size="sm"
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline justify-between gap-2">
                    <span className="truncate text-sm font-medium text-foreground">
                      {agent.name}
                    </span>
                    <span className="shrink-0 text-xs-plus text-foreground-faint">
                      {formatConversationTime(agent.updatedAt)}
                    </span>
                  </div>
                  <p className="mt-1 truncate text-xs text-foreground-subtle">
                    {agent.currentTask ?? agent.role}
                  </p>
                </div>
              </button>
            ))
          )}
        </nav>
      </div>
      <UsageIndicator usage={usage} />
      <div className="flex items-center gap-0.5 border-t border-border-subtle px-3 py-2.5">
        <span
          className="mr-auto flex items-center gap-2 pl-1 text-xs-plus text-foreground-faint"
          title="Local runtime"
        >
          <span className="size-1.5 rounded-full bg-status-success" /> Local runtime
        </span>
        {footerNavigation.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => onNavigate(id)}
            aria-label={label}
            title={label}
            aria-current={active === id ? "page" : undefined}
            className={cn(
              "relative grid size-7 place-items-center rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              active === id
                ? "bg-accent text-foreground"
                : "text-foreground-subtle hover:bg-muted hover:text-foreground"
            )}
          >
            <Icon size={15} strokeWidth={1.8} aria-hidden="true" />
            {id === "approvals" && (
              <span className="absolute right-1 top-1 size-1.5 rounded-full bg-status-waiting" />
            )}
          </button>
        ))}
        <ThemeToggle />
      </div>
    </aside>
  );
}

function SectionHeading({
  label,
  actionLabel,
  onAction
}: {
  label: string;
  actionLabel: string;
  onAction: () => void;
}) {
  return (
    <div className="flex h-7 items-center justify-between pl-2">
      <h2 className="text-2xs font-medium uppercase tracking-widest text-foreground-faint">
        {label}
      </h2>
      <button
        type="button"
        onClick={onAction}
        aria-label={actionLabel}
        title={actionLabel}
        className="grid size-6 place-items-center rounded-md text-foreground-faint transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <Plus size={13} strokeWidth={1.8} />
      </button>
    </div>
  );
}
