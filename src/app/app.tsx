import { useCallback, useEffect, useState } from "react";
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
import { useTheme } from "@/hooks/use-theme";
import { createAgent, listAgents, listProviders } from "@/lib/desktop-api";
import { demoApprovals, demoEvents, demoProviders, demoTasks } from "@/lib/demo-data";
import type { Agent, ApprovalRequest, NewAgentInput, ProviderSummary } from "@/types/domain";

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
  const [approvals, setApprovals] = useState<ApprovalRequest[]>(demoApprovals);
  const updater = useAppUpdater();
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
      setError(
        caught instanceof Error
          ? caught.message
          : "The local application state could not be loaded."
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);
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
  function selectAgent(agent: Agent) {
    setSelectedAgentId(agent.id);
    setView("chat");
  }
  function resolveApproval(id: string, status: "approved" | "denied") {
    setApprovals((current) =>
      current.map((approval) => (approval.id === id ? { ...approval, status } : approval))
    );
  }

  const selectedAgent = agents.find((agent) => agent.id === selectedAgentId);
  let page: React.ReactNode;
  if (view === "tasks") page = <TasksPage tasks={demoTasks} agents={agents} />;
  else if (view === "activity") page = <ActivityPage events={demoEvents} />;
  else if (view === "approvals")
    page = <ApprovalsPage approvals={approvals} onResolve={resolveApproval} />;
  else if (view === "settings") page = <SettingsPage providers={providers} updater={updater} />;

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
      />
      <main className="flex min-w-0 flex-1 flex-col">
        <UpdateBanner updater={updater} />
        {page ? (
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="mx-auto max-w-6xl px-8 py-8">{page}</div>
          </div>
        ) : selectedAgent ? (
          <AgentConversation key={selectedAgent.id} agent={selectedAgent} />
        ) : (
          <NoConversation onCreate={() => setCreateOpen(true)} />
        )}
      </main>
      <NewAgentDialog
        open={createOpen}
        busy={creating}
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
