import { useState, type FormEvent } from "react";
import { X } from "lucide-react";
import { AgentAvatar } from "./agent-avatar";
import { ModelPicker, type ModelChoice } from "./model-picker";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  agentColors,
  type AgentColor,
  type NewAgentInput,
  type ProviderSummary
} from "@/types/domain";

const providerDefault: ModelChoice = { model: null, reasoningEffort: null };

interface NewAgentDialogProps {
  open: boolean;
  busy: boolean;
  providers: ProviderSummary[];
  onClose: () => void;
  onSubmit: (input: NewAgentInput) => Promise<void>;
}

export function NewAgentDialog({ open, busy, providers, onClose, onSubmit }: NewAgentDialogProps) {
  const [color, setColor] = useState<AgentColor>("indigo");
  const [error, setError] = useState<string>();
  const [chosenProviderId, setChosenProviderId] = useState<string>();
  const [modelChoice, setModelChoice] = useState<ModelChoice>(providerDefault);
  if (!open) return null;
  const providerId = chosenProviderId ?? defaultProvider(providers);
  const provider = providers.find((candidate) => candidate.id === providerId);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    const data = new FormData(event.currentTarget);
    try {
      await onSubmit({
        name: formText(data, "name"),
        role: formText(data, "role"),
        description: formText(data, "description"),
        providerId: formText(data, "providerId") || "mock",
        identityColor: color,
        workspace: formText(data, "workspace"),
        instructions: formText(data, "instructions"),
        model: modelChoice.model,
        reasoningEffort: modelChoice.reasoningEffort
      });
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The agent could not be created.");
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-overlay p-5 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-agent-title"
    >
      <div className="max-h-[92vh] w-full max-w-xl overflow-y-auto rounded-xl border border-border bg-card shadow-panel">
        <div className="flex items-center justify-between border-b border-border-subtle px-5 py-4">
          <div>
            <h2 id="new-agent-title" className="font-semibold text-foreground">
              Create agent
            </h2>
            <p className="mt-0.5 text-xs text-foreground-subtle">
              Define a persistent identity backed by a local provider.
            </p>
          </div>
          <Button variant="ghost" size="icon" onClick={onClose} aria-label="Close">
            <X size={16} />
          </Button>
        </div>
        <form className="space-y-4 p-5" onSubmit={(event) => void handleSubmit(event)}>
          <div className="grid grid-cols-[auto_1fr] items-end gap-4">
            <AgentAvatar color={color} variant="orbital" status="idle" size="lg" />
            <div>
              <label
                className="mb-1.5 block text-xs font-medium text-foreground-muted"
                htmlFor="name"
              >
                Name
              </label>
              <Input id="name" name="name" required minLength={2} placeholder="Atlas" autoFocus />
            </div>
          </div>
          <Field label="Role">
            <Input name="role" required placeholder="Backend Engineer" />
          </Field>
          <Field label="Description">
            <textarea
              name="description"
              rows={2}
              className="field-textarea"
              placeholder="What this agent is responsible for."
            />
          </Field>
          <div className="grid grid-cols-2 gap-4">
            <Field label="Provider">
              <select
                name="providerId"
                className="field-select"
                value={providerId}
                onChange={(event) => {
                  setChosenProviderId(event.target.value);
                  setModelChoice(providerDefault);
                }}
              >
                {providers.map((provider) => (
                  <option
                    key={provider.id}
                    value={provider.id}
                    disabled={provider.status === "not-installed"}
                  >
                    {provider.name}
                    {provider.status === "available" ? "" : ` (${providerStatusLabel(provider)})`}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Workspace">
              <Input name="workspace" required placeholder="/path/to/workspace" />
            </Field>
          </div>
          <ModelPicker
            provider={provider}
            model={modelChoice.model}
            reasoningEffort={modelChoice.reasoningEffort}
            onChange={setModelChoice}
          />
          <Field label="Identity color">
            <div className="flex gap-2">
              {agentColors.map((option) => (
                <button
                  key={option}
                  type="button"
                  onClick={() => setColor(option)}
                  aria-label={`${option} identity`}
                  className={cn(
                    "size-7 rounded-full border-2 transition",
                    swatches[option],
                    color === option
                      ? "border-foreground scale-110"
                      : "border-transparent opacity-70 hover:opacity-100"
                  )}
                />
              ))}
            </div>
          </Field>
          <Field label="Instructions">
            <textarea
              name="instructions"
              rows={4}
              className="field-textarea"
              placeholder="Operating principles and durable instructions for this agent."
            />
          </Field>
          {error && (
            <p className="text-xs text-danger" role="alert">
              {error}
            </p>
          )}
          <div className="flex justify-end gap-2 border-t border-border-subtle pt-4">
            <Button type="button" variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={busy}>
              {busy ? "Creating…" : "Create Agent"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}

function formText(data: FormData, field: string): string {
  const value = data.get(field);
  return typeof value === "string" ? value.trim() : "";
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-xs font-medium text-foreground-muted">{label}</span>
      {children}
    </label>
  );
}

const swatches: Record<AgentColor, string> = {
  indigo: "bg-identity-indigo",
  cyan: "bg-identity-cyan",
  emerald: "bg-identity-emerald",
  amber: "bg-identity-amber",
  rose: "bg-identity-rose",
  violet: "bg-identity-violet"
};

/** Prefers an available real provider over the mock. */
function defaultProvider(providers: ProviderSummary[]): string {
  const real = providers.find(
    (provider) => provider.kind !== "mock" && provider.status === "available"
  );
  return real?.id ?? providers[0]?.id ?? "mock";
}

function providerStatusLabel(provider: ProviderSummary): string {
  return provider.status === "not-installed" ? "not installed" : "sign-in unconfirmed";
}
