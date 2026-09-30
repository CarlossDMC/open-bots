import { useCallback, useEffect, useState } from "react";
import { CommandPalette } from "./command-palette";
import { Sidebar, type ViewId } from "./sidebar";
import { ActivityPage } from "@/features/activity/activity-page";
import { AgentDetails } from "@/features/agents/agent-details";
import { AgentsPage } from "@/features/agents/agents-page";
import { NewAgentDialog } from "@/features/agents/new-agent-dialog";
import { ApprovalsPage } from "@/features/approvals/approvals-page";
import { SettingsPage } from "@/features/settings/settings-page";
import { TasksPage } from "@/features/tasks/tasks-page";
import { createAgent, listAgents, listProviders } from "@/lib/desktop-api";
import { demoApprovals, demoEvents, demoProviders, demoTasks } from "@/lib/demo-data";
import type { Agent, ApprovalRequest, NewAgentInput, ProviderSummary } from "@/types/domain";

export function App() {
  const [view, setView] = useState<ViewId>("agents");
  const [agents, setAgents] = useState<Agent[]>([]);
  const [providers, setProviders] = useState<ProviderSummary[]>(demoProviders);
  const [selectedAgent, setSelectedAgent] = useState<Agent>();
  const [isDemo, setIsDemo] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [createOpen, setCreateOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [approvals, setApprovals] = useState<ApprovalRequest[]>(demoApprovals);

  const load = useCallback(async () => {
    try {
      const [agentResult, providerResult] = await Promise.all([listAgents(), listProviders()]);
      setAgents(agentResult.agents);
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
      setCreateOpen(false);
    } finally {
      setCreating(false);
    }
  }
  function navigate(next: ViewId) {
    setView(next);
    setSelectedAgent(undefined);
  }
  function resolveApproval(id: string, status: "approved" | "denied") {
    setApprovals((current) =>
      current.map((approval) => (approval.id === id ? { ...approval, status } : approval))
    );
  }

  let content: React.ReactNode;
  if (loading) content = <div className="text-sm text-zinc-600">Loading local state…</div>;
  else if (error)
    content = (
      <div className="rounded-md border border-red-900/60 bg-red-950/20 p-4 text-sm text-red-300">
        <p>{error}</p>
        <button className="mt-2 text-xs underline" onClick={() => void load()}>
          Try again
        </button>
      </div>
    );
  else if (selectedAgent)
    content = <AgentDetails agent={selectedAgent} onBack={() => setSelectedAgent(undefined)} />;
  else if (view === "agents")
    content = (
      <AgentsPage
        agents={agents}
        isDemo={isDemo}
        onCreate={() => setCreateOpen(true)}
        onSelect={setSelectedAgent}
      />
    );
  else if (view === "tasks") content = <TasksPage tasks={demoTasks} agents={agents} />;
  else if (view === "activity") content = <ActivityPage events={demoEvents} />;
  else if (view === "approvals")
    content = <ApprovalsPage approvals={approvals} onResolve={resolveApproval} />;
  else content = <SettingsPage providers={providers} />;

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <Sidebar active={view} onNavigate={navigate} onOpenPalette={() => setPaletteOpen(true)} />
      <main className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-6xl px-8 py-8">{content}</div>
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
        onNavigate={navigate}
        onCreateAgent={() => setCreateOpen(true)}
      />
    </div>
  );
}
