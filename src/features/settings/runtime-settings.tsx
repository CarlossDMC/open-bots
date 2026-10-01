import { Loader2 } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { getRuntimeSettings, isTauriRuntime, updateRuntimeSettings } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";

const minChainTurns = 1;
const maxChainTurns = 50;

export function RuntimeSettingsForm() {
  const available = isTauriRuntime();
  const [value, setValue] = useState("");
  const [saved, setSaved] = useState<number>();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => {
    getRuntimeSettings()
      .then((settings) => {
        setSaved(settings.maxChainTurns);
        setValue(String(settings.maxChainTurns));
      })
      .catch((caught: unknown) =>
        setError(describeError(caught, "Runtime settings could not be loaded."))
      );
  }, []);

  const parsed = Number(value);
  const valid = Number.isInteger(parsed) && parsed >= minChainTurns && parsed <= maxChainTurns;
  const canSave = available && valid && parsed !== saved && !saving;

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSave) return;
    setSaving(true);
    try {
      const settings = await updateRuntimeSettings({ maxChainTurns: parsed });
      setSaved(settings.maxChainTurns);
      setError(undefined);
    } catch (caught) {
      setError(describeError(caught, "Runtime settings could not be saved."));
    } finally {
      setSaving(false);
    }
  }

  return (
    <form
      className="flex items-center justify-between gap-3 border-b border-border-subtle py-2.5"
      onSubmit={(event) => void handleSubmit(event)}
    >
      <label htmlFor="max-chain-turns" className="text-xs text-foreground-subtle">
        Chained turns without a message
        <span className="block text-2xs text-foreground-faint">
          Agents woken by tasks, messages, or routines stop after this many turns in a row
        </span>
        {error ? (
          <span role="alert" className="block text-2xs text-danger-foreground">
            {error}
          </span>
        ) : null}
      </label>
      <div className="flex shrink-0 items-center gap-2">
        <Input
          id="max-chain-turns"
          type="number"
          className="h-8 w-20"
          min={minChainTurns}
          max={maxChainTurns}
          value={value}
          disabled={!available}
          onChange={(event) => setValue(event.target.value)}
        />
        <Button type="submit" size="sm" variant="secondary" disabled={!canSave}>
          {saving ? <Loader2 size={13} className="animate-spin" /> : null}
          Save
        </Button>
      </div>
    </form>
  );
}
