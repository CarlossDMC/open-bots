import { useCallback, useEffect, useMemo, useState } from "react";
import { CommandPalette } from "./command-palette";
import { Sidebar, type ViewId } from "./sidebar";
import { ActivityPage } from "@/features/activity/activity-page";
import { NewAgentDialog } from "@/features/agents/new-agent-dialog";
import { AgentConversation, NoConversation } from "@/features/chat/agent-conversation";
import { ApprovalsPage } from "@/features/approvals/approvals-page";
import { SettingsPage } from "@/features/settings/settings-page";
import { TasksPage } from "@/features/tasks/tasks-page";
import { UpdateBanner } from "@/features/updates/update-banner";
import { useAppUpdater } from "@/hooks/use-app-updater";
import { useNotifications } from "@/hooks/use-notifications";
import { useProviderUsage } from "@/hooks/use-provider-usage";
import { useRuntimeEvents } from "@/hooks/use-runtime-events";
import { useTheme } from "@/hooks/use-theme";
import { mergeRuntimeEvent, toActivityEvent } from "@/lib/activity";
import { describeError } from "@/lib/utils";
import {
  createAgent,
  listAgents,
  listApprovals,
  listEvents,
  listProviders,
  resolveApproval
} from "@/lib/desktop-api";
import { demoProviders } from "@/lib/demo-data";
import type {
  Agent,
  ApprovalDecision,
  ApprovalRequest,
  NewAgentInput,
  ProviderSummary,
  RuntimeEvent
} from "@/types/domain";

const activityLimit = 200;

export function App() {
  const [view, setView] = useState<ViewId>("chat");
  const [agents, setAgents] = useState<Agent[]>([]);
  const [providers, setProviders] = useState<ProviderSummary[]>(demoProviders);
  const [selectedAgentId, setSelectedAgentId] = useState<string>();
  const [isDemo, setIsDemo] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [createOpen, setCreateOpen] = useState(false);
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

  const load = useCallback(async () => {
    try {
      const [agentResult, providerResult] = await Promise.all([listAgents(), listProviders()]);
      setAgents(agentResult.agents);
      setSelectedAgentId((current) => current ?? agentResult.agents[0]?.id);
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

  async function handleCreate(input: NewAgentInput) {
    setCreating(true);
    try {
      const agent = await createAgent(input);
      setAgents((current) => [agent, ...current]);
      setSelectedAgentId(agent.id);
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
    setSelectedAgentId(agent.id);
    setView("chat");
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

  const selectedAgent = agents.find((agent) => agent.id === selectedAgentId);
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
        selectedAgentId={selectedAgentId}
        isDemo={isDemo}
        loading={loading}
        error={error}
        onNavigate={setView}
        onSelectAgent={selectAgent}
        onCreateAgent={() => setCreateOpen(true)}
        onRetry={() => void load()}
        usage={usage}
      />
      <main className="flex min-w-0 flex-1 flex-col">
        <UpdateBanner updater={updater} />
        {page ? (
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="mx-auto max-w-6xl px-8 py-8">{page}</div>
          </div>
        ) : selectedAgent ? (
          <AgentConversation
            key={selectedAgent.id}
            agent={selectedAgent}
            agents={agents}
            provider={providers.find((provider) => provider.id === selectedAgent.providerId)}
            onAgentUpdated={replaceAgent}
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
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onNavigate={setView}
        onCreateAgent={() => setCreateOpen(true)}
        onToggleTheme={toggleTheme}
      />
    </div>
  );
}
