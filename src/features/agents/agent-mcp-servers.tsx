import { Plug, Plus, X } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { updateAgentMcpServers } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { Agent } from "@/types/domain";

/** Saves each change immediately; it applies from the agent's next turn. */
export function AgentMcpServersSection({
  agent,
  onAgentUpdated
}: {
  agent: Agent;
  onAgentUpdated?: (agent: Agent) => void;
}) {
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const servers = agent.mcpServers ?? [];
  const working = agent.status === "working";
  const disabled = saving || working;

  async function save(next: string[]) {
    setSaving(true);
    try {
      onAgentUpdated?.(await updateAgentMcpServers(agent, next));
      setError(undefined);
      return true;
    } catch (caught) {
      setError(describeError(caught, "The MCP servers could not be saved."));
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function add(event: FormEvent) {
    event.preventDefault();
    const name = draft.trim();
    if (!name || disabled) return;
    if (servers.includes(name)) {
      setDraft("");
      return;
    }
    if (await save([...servers, name])) setDraft("");
  }

  return (
    <section className="panel lg:col-span-5">
      <h2 className="section-title">
        <Plug size={14} /> MCP servers
      </h2>
      {servers.length > 0 ? (
        <ul className="flex flex-wrap gap-2" aria-label="Selected MCP servers">
          {servers.map((server) => (
            <li
              key={server}
              className="flex items-center gap-1 rounded border border-border-subtle bg-card py-1 pl-2.5 pr-1 text-xs text-foreground-secondary"
            >
              {server}
              <button
                type="button"
                className="rounded p-0.5 text-foreground-faint hover:text-foreground disabled:cursor-not-allowed"
                aria-label={`Remove ${server}`}
                title={`Remove ${server}`}
                disabled={disabled}
                onClick={() => void save(servers.filter((name) => name !== server))}
              >
                <X size={12} />
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-sm text-foreground-muted">
          No MCP servers. The agent only uses Open Bots runtime tools.
        </p>
      )}
      <form className="mt-3 flex max-w-md gap-2" onSubmit={(event) => void add(event)}>
        <Input
          value={draft}
          disabled={disabled}
          aria-label="MCP server name"
          placeholder="claude.ai Atlassian"
          onChange={(event) => setDraft(event.target.value)}
          className="h-8"
        />
        <Button type="submit" size="sm" variant="secondary" disabled={disabled || !draft.trim()}>
          <Plus size={14} /> Add
        </Button>
      </form>
      <p className="mt-2 text-xs text-foreground-faint">
        Use server names from the provider's own configuration, as <code>claude mcp list</code>{" "}
        prints them. Every tool of a selected server runs without approval, including tools that
        create or change data in external services.{" "}
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
