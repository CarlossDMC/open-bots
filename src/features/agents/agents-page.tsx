import { ArrowRight, Plus, Users } from "lucide-react";
import { AgentAvatar } from "./agent-avatar";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/ui/status-badge";
import type { Agent } from "@/types/domain";

interface AgentsPageProps {
  agents: Agent[];
  isDemo: boolean;
  onCreate: () => void;
  onSelect: (agent: Agent) => void;
}

export function AgentsPage({ agents, isDemo, onCreate, onSelect }: AgentsPageProps) {
  return (
    <div className="animate-fade-in">
      <header className="mb-7 flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold tracking-tight text-zinc-100">Agents</h1>
          <p className="mt-1 text-sm text-zinc-500">
            Persistent identities connected to interchangeable providers.
          </p>
        </div>
        <Button onClick={onCreate}>
          <Plus size={15} /> New Agent
        </Button>
      </header>
      {isDemo && (
        <div className="mb-4 rounded-md border border-amber-900/50 bg-amber-950/20 px-3 py-2 text-xs text-amber-300/80">
          Browser preview uses demonstration data. Run the Tauri app for local SQLite persistence.
        </div>
      )}
      {agents.length === 0 ? (
        <div className="grid min-h-72 place-items-center rounded-lg border border-dashed border-zinc-800">
          <div className="text-center">
            <Users className="mx-auto mb-3 text-zinc-600" size={24} />
            <p className="text-sm text-zinc-300">No agents yet</p>
            <p className="mt-1 text-xs text-zinc-600">
              Create an agent to establish its identity and workspace.
            </p>
          </div>
        </div>
      ) : (
        <div className="overflow-hidden rounded-lg border border-zinc-800/80 bg-zinc-950/35">
          {agents.map((agent) => (
            <button
              key={agent.id}
              onClick={() => onSelect(agent)}
              className="group flex w-full items-center gap-4 border-b border-zinc-900 px-4 py-3.5 text-left transition-colors last:border-0 hover:bg-zinc-900/60"
            >
              <AgentAvatar
                color={agent.identityColor}
                variant={agent.avatarVariant}
                status={agent.status}
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-zinc-100">{agent.name}</span>
                  <StatusBadge status={agent.status} />
                </div>
                <div className="mt-0.5 flex items-center gap-2 text-xs text-zinc-500">
                  <span>{agent.role}</span>
                  <span className="text-zinc-700">/</span>
                  <span>{agent.providerId === "mock" ? "Mock Provider" : agent.providerId}</span>
                </div>
              </div>
              <div className="hidden max-w-64 truncate text-xs text-zinc-500 md:block">
                {agent.currentTask ?? "No active task"}
              </div>
              <ArrowRight
                size={15}
                className="text-zinc-700 transition group-hover:translate-x-0.5 group-hover:text-zinc-400"
              />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
