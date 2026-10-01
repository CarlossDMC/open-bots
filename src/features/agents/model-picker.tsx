import { useId } from "react";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
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
  const modelOptions: ComboboxOption[] = [
    { value: "", label: catalog.loading ? "Loading models…" : "Provider default" },
    ...catalog.models.map((candidate) => ({
      value: candidate.id,
      label: candidate.isDefault ? `${candidate.displayName} (default)` : candidate.displayName,
      description: candidate.description || undefined
    })),
    ...(model && !catalog.loading && !selected
      ? [{ value: model, label: `${model} (not in catalog)` }]
      : [])
  ];
  const effortOptions: ComboboxOption[] = [
    {
      value: "",
      label: selected?.defaultReasoningEffort
        ? `Model default (${selected.defaultReasoningEffort})`
        : "Model default"
    },
    ...efforts.map((effort) => ({ value: effort, label: effort })),
    ...(reasoningEffort && !efforts.includes(reasoningEffort)
      ? [{ value: reasoningEffort, label: reasoningEffort }]
      : [])
  ];

  return (
    <div className={cn("grid grid-cols-2 gap-4", className)}>
      <div>
        <label className="mb-1.5 block text-xs font-medium text-foreground-muted" htmlFor={modelId}>
          Model
        </label>
        <Combobox
          id={modelId}
          value={model ?? ""}
          disabled={busy}
          searchPlaceholder="Search models…"
          options={modelOptions}
          onChange={(value) => onChange({ model: value || null, reasoningEffort: null })}
        />
      </div>
      <div>
        <label
          className="mb-1.5 block text-xs font-medium text-foreground-muted"
          htmlFor={effortId}
        >
          Reasoning
        </label>
        <Combobox
          id={effortId}
          value={reasoningEffort ?? ""}
          disabled={busy || !model}
          searchPlaceholder="Search efforts…"
          options={effortOptions}
          onChange={(value) => onChange({ model, reasoningEffort: value || null })}
        />
      </div>
      {catalog.error && (
        <p className="col-span-2 text-xs text-danger-foreground" role="alert">
          {catalog.error}
        </p>
      )}
    </div>
  );
}
