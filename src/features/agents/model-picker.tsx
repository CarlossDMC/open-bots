import { useId } from "react";
import { useProviderModels } from "@/hooks/use-provider-models";
import { supportsModelSelection } from "@/lib/models";
import { cn } from "@/lib/utils";
import type { ProviderSummary } from "@/types/domain";

export interface ModelChoice {
  model: string | null;
  reasoningEffort: string | null;
}

interface ModelPickerProps extends ModelChoice {
  provider?: ProviderSummary;
  disabled?: boolean;
  className?: string;
  onChange: (choice: ModelChoice) => void;
}

/** Model and reasoning-effort selects backed by the provider's own catalog. */
export function ModelPicker({
  provider,
  model,
  reasoningEffort,
  disabled = false,
  className,
  onChange
}: ModelPickerProps) {
  const supported = supportsModelSelection(provider);
  const catalog = useProviderModels(supported ? provider?.id : undefined);
  const modelId = useId();
  const effortId = useId();

  if (!provider) return null;
  if (!supported) {
    return (
      <p className={cn("text-xs text-foreground-faint", className)}>
        Model selection is not supported by {provider.name}.
      </p>
    );
  }

  const selected = catalog.models.find((candidate) => candidate.id === model);
  const efforts = selected?.reasoningEfforts ?? [];
  const busy = disabled || catalog.loading;

  return (
    <div className={cn("grid grid-cols-2 gap-4", className)}>
      <div>
        <label className="mb-1.5 block text-xs font-medium text-foreground-muted" htmlFor={modelId}>
          Model
        </label>
        <select
          id={modelId}
          className="field-select"
          value={model ?? ""}
          disabled={busy}
          onChange={(event) =>
            onChange({ model: event.target.value || null, reasoningEffort: null })
          }
        >
          <option value="">{catalog.loading ? "Loading models…" : "Provider default"}</option>
          {catalog.models.map((candidate) => (
            <option key={candidate.id} value={candidate.id}>
              {candidate.displayName}
              {candidate.isDefault ? " (default)" : ""}
            </option>
          ))}
          {model && !catalog.loading && !selected && (
            <option value={model}>{model} (not in catalog)</option>
          )}
        </select>
      </div>
      <div>
        <label
          className="mb-1.5 block text-xs font-medium text-foreground-muted"
          htmlFor={effortId}
        >
          Reasoning
        </label>
        <select
          id={effortId}
          className="field-select"
          value={reasoningEffort ?? ""}
          disabled={busy || !model}
          onChange={(event) => onChange({ model, reasoningEffort: event.target.value || null })}
        >
          <option value="">
            {selected?.defaultReasoningEffort
              ? `Model default (${selected.defaultReasoningEffort})`
              : "Model default"}
          </option>
          {efforts.map((effort) => (
            <option key={effort} value={effort}>
              {effort}
            </option>
          ))}
          {reasoningEffort && !efforts.includes(reasoningEffort) && (
            <option value={reasoningEffort}>{reasoningEffort}</option>
          )}
        </select>
      </div>
      {catalog.error && (
        <p className="col-span-2 text-xs text-danger-foreground" role="alert">
          {catalog.error}
        </p>
      )}
    </div>
  );
}
