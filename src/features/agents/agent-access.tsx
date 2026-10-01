import { Globe, Loader2, SquareTerminal } from "lucide-react";
import { useState } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { updateAgentAccess } from "@/lib/desktop-api";
import { canUseInternet, canWriteWorkspace } from "@/lib/permissions";
import { describeError } from "@/lib/utils";
import type { Agent } from "@/types/domain";

/** Saves each change immediately; it applies from the agent's next turn. */
export function AgentAccessSection({
  agent,
  onAgentUpdated
}: {
  agent: Agent;
  onAgentUpdated?: (agent: Agent) => void;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const working = agent.status === "working";
  const write = canWriteWorkspace(agent.permissions);
  const internet = canUseInternet(agent.permissions);

  async function save(nextWrite: boolean, nextInternet: boolean) {
    setSaving(true);
    try {
      onAgentUpdated?.(await updateAgentAccess(agent, nextWrite, nextInternet));
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "Access could not be saved."));
    } finally {
      setSaving(false);
    }
  }

  const disabled = saving || working;
  return (
    <section className="panel lg:col-span-5">
      <h2 className="section-title">
        Access {saving ? <Loader2 size={13} className="animate-spin" aria-hidden="true" /> : null}
      </h2>
      <div className="grid gap-3 md:grid-cols-2">
        <AccessToggle
          icon={SquareTerminal}
          label="Edit files and run commands"
          description="Changes files and runs shell commands in the workspace without asking."
          checked={write}
          disabled={disabled}
          onChange={(checked) => void save(checked, internet)}
        />
        <AccessToggle
          icon={Globe}
          label="Internet access"
          description="Searches the web and opens pages. With commands on, they can reach the network too."
          checked={internet}
          disabled={disabled}
          onChange={(checked) => void save(write, checked)}
        />
      </div>
      <p
        role={error ? "alert" : undefined}
        className={
          error ? "mt-3 text-xs text-danger-foreground" : "mt-3 text-xs text-foreground-faint"
        }
      >
        {error ??
          (working
            ? "Access can change once the current turn ends."
            : "Changes apply from the agent's next turn.")}
      </p>
    </section>
  );
}

function AccessToggle({
  icon: Icon,
  label,
  description,
  checked,
  disabled,
  onChange
}: {
  icon: typeof Globe;
  label: string;
  description: string;
  checked: boolean;
  disabled: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-3 rounded-md border border-border-subtle bg-card px-3 py-2.5">
      <Checkbox
        checked={checked}
        disabled={disabled}
        aria-label={label}
        onCheckedChange={onChange}
        className="mt-0.5"
      />
      <span className="min-w-0">
        <span className="flex items-center gap-1.5 text-sm text-foreground">
          <Icon size={13} className="text-foreground-subtle" aria-hidden="true" />
          {label}
        </span>
        <span className="mt-0.5 block text-xs text-foreground-subtle">{description}</span>
      </span>
    </label>
  );
}
