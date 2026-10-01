import { useCallback, useEffect, useMemo, useState } from "react";
import { CommandPalette } from "./command-palette";
import { Sidebar, type ViewId } from "./sidebar";
import { ActivityPage } from "@/features/activity/activity-page";
import { NewAgentDialog } from "@/features/agents/new-agent-dialog";
import { AgentConversation, NoConversation } from "@/features/chat/agent-conversation";
import { ApprovalsPage } from "@/features/approvals/approvals-page";
import { GroupConversation } from "@/features/groups/group-conversation";
import { NewGroupDialog } from "@/features/groups/new-group-dialog";
import { SettingsPage } from "@/features/settings/settings-page";
import { TasksPage } from "@/features/tasks/tasks-page";
import { UpdateBanner } from "@/features/updates/update-banner";
import { useAppUpdater } from "@/hooks/use-app-updater";
import { useGroups } from "@/hooks/use-groups";
import { useNotifications } from "@/hooks/use-notifications";
import { useProviderUsage } from "@/hooks/use-provider-usage";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { useTheme } from "@/hooks/use-theme";
import { mergeRuntimeEvent, toActivityEvent } from "@/lib/activity";
import { describeError } from "@/lib/utils";
import {
  createAgent,
  deleteAgent,
  listAgents,
  listApprovals,
  listEvents,
  isTauriRuntime,
  listProviders,
  resolveApproval
} from "@/lib/desktop-api";
import { demoProviders } from "@/lib/demo-data";
import type {
  Agent,
  ApprovalDecision,
  ApprovalRequest,
  Group,
  NewAgentInput,
  NewGroupInput,
  ProviderSummary,
  RuntimeEvent
} from "@/types/domain";

const activityLimit = 200;

/** The conversation shown in the chat view: an agent's own thread or a group. */
type Selection = { kind: "agent"; id: string } | { kind: "group"; id: string };

export function App() {
  const [view, setView] = useState<ViewId>("chat");
  const [agents, setAgents] = useState<Agent[]>([]);
  const [providers, setProviders] = useState<ProviderSummary[]>(demoProviders);
  const [selection, setSelection] = useState<Selection>();
  const [isDemo, setIsDemo] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [createOpen, setCreateOpen] = useState(false);
  const [createGroupOpen, setCreateGroupOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [approvals, setApprovals] = useState<ApprovalRequest[]>([]);
  const [approvalError, setApprovalError] = useState<string>();
  const [events, setEvents] = useState<RuntimeEvent[]>([]);
  const [activityError, setActivityError] = useState<string>();
  const updater = useAppUpdater();
  const notifications = useNotifications();
  const usage = useProviderUsage();
  const { toggle: toggleTheme } = useTheme();
  const groups = useGroups();

  const load = useCallback(async () => {
    try {
      const [agentResult, providerResult] = await Promise.all([listAgents(), listProviders()]);
      setAgents(agentResult.agents);
      setSelection((current) => {
        if (current) return current;
        const first = agentResult.agents[0];
        return first ? { kind: "agent", id: first.id } : undefined;
      });
      setIsDemo(agentResult.isDemo);
      setProviders(providerResult);
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The local application state could not be loaded."));
    } finally {
      setLoading(false);
    }
  }, []);

  /** Refreshes agent status without re-running provider detection. */
  const reloadAgents = useCallback(async () => {
    try {
      setAgents((await listAgents()).agents);
    } catch (caught) {
      setError(describeError(caught, "Agents could not be reloaded."));
    }
  }, []);

  const loadApprovals = useCallback(async () => {
    try {
      setApprovals(await listApprovals());
      setApprovalError(undefined);
    } catch (caught) {
      setApprovalError(describeError(caught, "Approvals could not be loaded."));
    }
  }, []);

  const loadActivity = useCallback(async () => {
    try {
      setEvents(await listEvents(activityLimit));
      setActivityError(undefined);
    } catch (caught) {
      setActivityError(describeError(caught, "Activity could not be loaded."));
    }
  }, []);

  useEffect(() => {
    void load();
    void loadApprovals();
    void loadActivity();
  }, [load, loadApprovals, loadActivity]);

  useRuntimeEvents((event) => {
    setEvents((current) => mergeRuntimeEvent(current, event, activityLimit));
    if (event.eventType.startsWith("approval.")) void loadApprovals();
    if (event.eventType.startsWith("agent.")) void reloadAgents();
    notifications.notifyFor(event, agents);
  });
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((current) => !current);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  async function handleCreate(input: NewAgentInput, mcpServers: string[]) {
    setCreating(true);
    try {
      const agent = await createAgent(input, mcpServers);
      setAgents((current) => [agent, ...current]);
      setSelection({ kind: "agent", id: agent.id });
      setView("chat");
      setCreateOpen(false);
    } finally {
      setCreating(false);
    }
  }
  function replaceAgent(updated: Agent) {
    setAgents((current) =>
      current.map((candidate) => (candidate.id === updated.id ? updated : candidate))
    );
  }
  function selectAgent(agent: Agent) {
    setSelection({ kind: "agent", id: agent.id });
    setView("chat");
  }
  function selectGroup(group: Group) {
    setSelection({ kind: "group", id: group.id });
    setView("chat");
  }
  async function handleCreateGroup(input: NewGroupInput) {
    const group = await groups.create(input);
    if (group) selectGroup(group);
    return Boolean(group);
  }
  async function handleDeleteAgent(agent: Agent) {
    try {
      await deleteAgent(agent.id);
    } catch (caught) {
      setError(describeError(caught, "The agent could not be deleted."));
      return false;
    }
    const remaining = agents.filter((candidate) => candidate.id !== agent.id);
    setAgents(remaining);
    setSelection((current) =>
      current?.kind === "agent" && current.id === agent.id
        ? remaining[0]
          ? { kind: "agent", id: remaining[0].id }
          : undefined
        : current
    );
    return true;
  }
  async function handleDeleteGroup(group: Group) {
    const deleted = await groups.remove(group.id);
    if (deleted) {
      setSelection((current) =>
        current?.kind === "group" && current.id === group.id
          ? agents[0]
            ? { kind: "agent", id: agents[0].id }
            : undefined
          : current
      );
    }
    return deleted;
  }
  async function handleResolveApproval(approval: ApprovalRequest, decision: ApprovalDecision) {
    try {
      const resolved = await resolveApproval(approval, decision);
      setApprovals((current) =>
        current.map((candidate) => (candidate.id === resolved.id ? resolved : candidate))
      );
      setApprovalError(undefined);
    } catch (caught) {
      setApprovalError(describeError(caught, "The decision could not be saved."));
    }
  }

  const selectedAgentId = selection?.kind === "agent" ? selection.id : undefined;
  const selectedGroupId = selection?.kind === "group" ? selection.id : undefined;
  const selectedAgent = agents.find((agent) => agent.id === selectedAgentId);
  const selectedGroup = groups.groups.find((group) => group.id === selectedGroupId);
  const activity = useMemo(
    () => events.map((event) => toActivityEvent(event, agents)),
    [events, agents]
  );
  let page: React.ReactNode;
  if (view === "tasks") page = <TasksPage agents={agents} />;
  else if (view === "activity") page = <ActivityPage events={activity} error={activityError} />;
  else if (view === "approvals")
    page = (
      <ApprovalsPage
        approvals={approvals}
        agents={agents}
        isDemo={isDemo}
        error={approvalError}
        onResolve={(approval, decision) => void handleResolveApproval(approval, decision)}
      />
    );
  else if (view === "settings")
    page = (
      <SettingsPage
        providers={providers}
        usage={usage}
        updater={updater}
        notifications={notifications}
      />
    );

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <Sidebar
        active={view}
        agents={agents}
        groups={groups.groups}
        selectedAgentId={selectedAgentId}
        selectedGroupId={selectedGroupId}
        isDemo={isDemo}
        loading={loading}
        error={error}
        onNavigate={setView}
        onSelectAgent={selectAgent}
        onSelectGroup={selectGroup}
        onCreateAgent={() => setCreateOpen(true)}
        onCreateGroup={() => setCreateGroupOpen(true)}
        onRetry={() => void load()}
        usage={usage}
      />
      <main className="flex min-w-0 flex-1 flex-col">
        <UpdateBanner updater={updater} />
        {page ? (
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="mx-auto max-w-6xl px-8 py-8">{page}</div>
          </div>
        ) : selectedGroup ? (
          <GroupConversation
            key={selectedGroup.id}
            group={selectedGroup}
            agents={agents}
            onDelete={handleDeleteGroup}
            onOpenAgent={selectAgent}
          />
        ) : selectedAgent ? (
          <AgentConversation
            key={selectedAgent.id}
            agent={selectedAgent}
            agents={agents}
            provider={providers.find((provider) => provider.id === selectedAgent.providerId)}
            onAgentUpdated={replaceAgent}
            onDelete={handleDeleteAgent}
          />
        ) : (
          <NoConversation onCreate={() => setCreateOpen(true)} />
        )}
      </main>
      <NewAgentDialog
        open={createOpen}
        busy={creating}
        providers={providers}
        onClose={() => setCreateOpen(false)}
        onSubmit={handleCreate}
      />
      <NewGroupDialog
        open={createGroupOpen}
        available={isTauriRuntime()}
        agents={agents}
        error={groups.error}
        onClose={() => setCreateGroupOpen(false)}
        onSubmit={handleCreateGroup}
      />
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onNavigate={setView}
        onCreateAgent={() => setCreateOpen(true)}
        onCreateGroup={() => setCreateGroupOpen(true)}
        onToggleTheme={toggleTheme}
      />
    </div>
  );
}
