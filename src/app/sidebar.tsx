import { Activity, CheckSquare2, ListTodo, Plus, Search, Settings } from "lucide-react";
import { useMemo, useState } from "react";
import { ThemeToggle } from "@/features/appearance/theme-toggle";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { cn, formatConversationTime } from "@/lib/utils";
import type { Agent } from "@/types/domain";

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
  selectedAgentId?: string;
  isDemo: boolean;
  loading: boolean;
  error?: string;
  onNavigate: (view: ViewId) => void;
  onSelectAgent: (agent: Agent) => void;
  onCreateAgent: () => void;
  onRetry: () => void;
}

export function Sidebar({
  active,
  agents,
  selectedAgentId,
  isDemo,
  loading,
  error,
  onNavigate,
  onSelectAgent,
  onCreateAgent,
  onRetry
}: SidebarProps) {
  const [query, setQuery] = useState("");
  const visibleAgents = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return agents;
    return agents.filter((agent) =>
      `${agent.name} ${agent.role}`.toLowerCase().includes(normalized)
    );
  }, [agents, query]);

  return (
    <aside className="flex w-72 shrink-0 flex-col border-r border-border-subtle bg-surface">
      <div className="flex h-11 items-center justify-between pl-4 pr-2">
        <span className="text-xs font-medium text-foreground-subtle">Open Bots</span>
        <button
          type="button"
          onClick={onCreateAgent}
          aria-label="New agent"
          title="New agent"
          className="grid size-7 place-items-center rounded-md text-foreground-muted transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <Plus size={16} strokeWidth={1.8} />
        </button>
      </div>
      <div className="px-3 pb-2">
        <label className="relative block">
          <span className="sr-only">Search agents</span>
          <Search
            size={14}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-foreground-faint"
            aria-hidden="true"
          />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search"
            className="h-8 w-full rounded-lg border border-border bg-card pl-8 pr-3 text-sm text-foreground outline-none placeholder:text-foreground-faint focus:border-border-strong focus:ring-1 focus:ring-ring/40"
          />
        </label>
        {isDemo && (
          <p className="mt-2 rounded-md border border-warning-border bg-warning-muted px-2 py-1.5 text-2xs text-warning-foreground">
            Browser preview uses demonstration data.
          </p>
        )}
      </div>
      <nav className="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-3" aria-label="Agents">
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
              aria-current={active === "chat" && agent.id === selectedAgentId ? "page" : undefined}
              className={cn(
                "flex h-14 w-full items-center gap-3 rounded-lg px-2.5 text-left transition-colors",
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
                  <span className="truncate text-sm text-foreground">{agent.name}</span>
                  <span className="shrink-0 text-2xs text-foreground-faint">
                    {formatConversationTime(agent.updatedAt)}
                  </span>
                </div>
                <p className="mt-0.5 truncate text-xs text-foreground-subtle">
                  {agent.currentTask ?? agent.role}
                </p>
              </div>
            </button>
          ))
        )}
      </nav>
      <div className="flex items-center gap-0.5 border-t border-border-subtle px-3 py-2">
        <span
          className="mr-auto flex items-center gap-2 pl-1 text-2xs text-foreground-faint"
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
              "relative grid size-6 place-items-center rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              active === id
                ? "bg-accent text-foreground"
                : "text-foreground-subtle hover:bg-muted hover:text-foreground"
            )}
          >
            <Icon size={13} strokeWidth={1.8} aria-hidden="true" />
            {id === "approvals" && (
              <span className="absolute right-0.5 top-0.5 size-1.5 rounded-full bg-status-waiting" />
            )}
          </button>
        ))}
        <ThemeToggle />
      </div>
    </aside>
  );
}
