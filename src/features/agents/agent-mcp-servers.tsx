import { Plug } from "lucide-react";
import { useState } from "react";
import { McpServerPicker } from "./mcp-server-picker";
import { updateAgentMcpServers } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { Agent, McpCatalogEntry, ProviderSummary } from "@/types/domain";

/** Saves each change immediately; it applies from the agent's next turn. */
export function AgentMcpServersSection({
  agent,
  provider,
  entries,
  onAgentUpdated
}: {
  agent: Agent;
  provider?: ProviderSummary;
  entries: McpCatalogEntry[];
  onAgentUpdated?: (agent: Agent) => void;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const working = agent.status === "working";
  // Servers removed from the catalog no longer reach the agent, so they are not shown.
  const selected = (agent.mcpServers ?? []).filter((name) =>
    entries.some((entry) => entry.providerId === agent.providerId && entry.name === name)
  );

  async function save(next: string[]) {
    setSaving(true);
    try {
      onAgentUpdated?.(await updateAgentMcpServers(agent, next));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "The MCP servers could not be saved."));
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="panel lg:col-span-5">
      <h2 className="section-title">
        <Plug size={14} /> MCP servers
      </h2>
      <McpServerPicker
        provider={provider}
        entries={entries}
        selected={selected}
        disabled={saving || working}
        onChange={(next) => void save(next)}
      />
      <p className="mt-2 text-xs text-foreground-faint">
        Every tool of a selected server runs without approval, including tools that create or change
        data in external services.{" "}
        {working
          ? "Servers can change once the current turn ends."
          : "Changes apply from the next turn."}
      </p>
      {error && (
        <p className="mt-2 text-xs text-danger-foreground" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
