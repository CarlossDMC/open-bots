import { Cpu, FolderGit2, Settings2 } from "lucide-react";
import { useState } from "react";
import { AgentAvatar } from "./agent-avatar";
import { AgentMcpServersSection } from "./agent-mcp-servers";
import { AgentMemorySection } from "./agent-memory";
import { AgentRoutinesSection } from "./agent-routines";
import { ModelPicker, type ModelChoice } from "./model-picker";
import { StatusBadge } from "@/components/ui/status-badge";
import { updateAgentModel } from "@/lib/desktop-api";
import { describeModel, supportsConfiguredMcpServers } from "@/lib/models";
import { describeError, titleCase } from "@/lib/utils";
import type { Agent, ProviderSummary } from "@/types/domain";

export function AgentDetails({
  agent,
  provider,
  onAgentUpdated
}: {
  agent: Agent;
  provider?: ProviderSummary;
  onAgentUpdated?: (agent: Agent) => void;
}) {
  return (
    <div className="animate-fade-in">
      <div className="flex items-center gap-4 border-b border-border-subtle pb-6">
        <AgentAvatar
          color={agent.identityColor}
          variant={agent.avatarVariant}
          seed={agent.id}
          status={agent.status}
          size="lg"
        />
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-xl font-semibold text-foreground">{agent.name}</h1>
            <StatusBadge status={agent.status} />
          </div>
          <p className="mt-1 text-sm text-foreground-muted">{agent.role}</p>
          <p className="mt-1 text-xs text-foreground-faint">{agent.description}</p>
        </div>
      </div>
      <div className="mt-6 grid gap-5 lg:grid-cols-5">
        <section className="panel lg:col-span-3">
          <h2 className="section-title">Overview</h2>
          <dl className="detail-grid">
            <Detail label="Provider" value={provider?.name ?? agent.providerId} />
            <Detail label="Model" value={describeModel(agent)} />
            <Detail label="Status" value={titleCase(agent.status)} />
            <Detail label="Workspace" value={agent.workspace} />
            <Detail label="Current task" value={agent.currentTask ?? "No active task"} />
          </dl>
        </section>
        <section className="panel lg:col-span-2">
          <h2 className="section-title">
            <FolderGit2 size={14} /> Workspace
          </h2>
          <p className="break-all text-sm text-foreground-secondary">{agent.workspace}</p>
          <p className="mt-2 text-xs text-foreground-faint">Local directory workspace</p>
        </section>
        <AgentModelSection agent={agent} provider={provider} onAgentUpdated={onAgentUpdated} />
        {supportsConfiguredMcpServers(provider) && (
          <AgentMcpServersSection agent={agent} onAgentUpdated={onAgentUpdated} />
        )}
        <section className="panel lg:col-span-5">
          <h2 className="section-title">
            <Settings2 size={14} /> Configuration
          </h2>
          <div className="grid gap-6 md:grid-cols-2">
            <div>
              <p className="label">Instructions</p>
              <p className="mt-2 whitespace-pre-wrap text-sm leading-6 text-foreground-secondary">
                {agent.instructions || "No persistent instructions."}
              </p>
            </div>
            <div>
              <p className="label">Permissions</p>
              <div className="mt-2 grid grid-cols-2 gap-2">
                {(Object.entries(agent.permissions) as Array<[string, string]>).map(
                  ([key, value]) => (
                    <div
                      key={key}
                      className="rounded border border-border-subtle bg-card px-2.5 py-2"
                    >
                      <p className="text-xs-plus text-foreground-faint">{titleCase(key)}</p>
                      <p className="mt-0.5 text-xs text-foreground-secondary">{titleCase(value)}</p>
                    </div>
                  )
                )}
              </div>
            </div>
          </div>
        </section>
        <AgentRoutinesSection agentId={agent.id} />
        <AgentMemorySection agentId={agent.id} />
      </div>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="label">{label}</dt>
      <dd className="mt-1 text-sm text-foreground-secondary">{value}</dd>
    </div>
  );
}

/** Saves each change immediately; it applies from the agent's next turn. */
function AgentModelSection({
  agent,
  provider,
  onAgentUpdated
}: {
  agent: Agent;
  provider?: ProviderSummary;
  onAgentUpdated?: (agent: Agent) => void;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const working = agent.status === "working";

  async function save(choice: ModelChoice) {
    setSaving(true);
    try {
      onAgentUpdated?.(await updateAgentModel(agent, choice.model, choice.reasoningEffort));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The model could not be saved."));
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="panel lg:col-span-5">
      <h2 className="section-title">
        <Cpu size={14} /> Model
      </h2>
      <ModelPicker
        provider={provider}
        model={agent.model ?? null}
        reasoningEffort={agent.reasoningEffort ?? null}
        disabled={saving || working}
        onChange={(choice) => void save(choice)}
      />
      <p className="mt-2 text-xs text-foreground-faint">
        {working
          ? "The model can change once the current turn ends."
          : "Changes apply from the next turn; the conversation continues."}
      </p>
      {error && (
        <p className="mt-2 text-xs text-danger-foreground" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
