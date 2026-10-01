import { Checkbox } from "@/components/ui/checkbox";
import type { McpCatalogEntry, ProviderSummary } from "@/types/domain";

/** Checkbox list of the catalog servers the agent's provider can load. */
export function McpServerPicker({
  provider,
  entries,
  selected,
  disabled = false,
  onChange
}: {
  provider?: ProviderSummary;
  entries: McpCatalogEntry[];
  selected: string[];
  disabled?: boolean;
  onChange: (servers: string[]) => void;
}) {
  const available = entries.filter((entry) => entry.providerId === provider?.id);
  const otherProviders = entries.length - available.length;

  function toggle(name: string, checked: boolean) {
    onChange(checked ? [...selected, name] : selected.filter((candidate) => candidate !== name));
  }

  return (
    <div>
      {available.length > 0 ? (
        <ul className="grid gap-1.5 sm:grid-cols-2" aria-label="MCP servers">
          {available.map((entry) => {
            const id = `mcp-server-${entry.name.replace(/\W+/g, "-")}`;
            return (
              <li key={entry.name} className="flex items-center gap-2">
                <Checkbox
                  id={id}
                  checked={selected.includes(entry.name)}
                  disabled={disabled}
                  onCheckedChange={(checked) => toggle(entry.name, checked)}
                />
                <label htmlFor={id} className="text-sm text-foreground-secondary">
                  {entry.name}
                </label>
              </li>
            );
          })}
        </ul>
      ) : (
        <p className="text-sm text-foreground-muted">
          No MCP servers in the catalog for {provider?.name ?? "this provider"}. Import them in
          Settings.
        </p>
      )}
      {otherProviders > 0 && (
        <p className="mt-2 text-xs text-foreground-faint">
          {otherProviders === 1
            ? "1 catalog server belongs"
            : `${otherProviders} catalog servers belong`}{" "}
          to another provider and cannot be loaded by this agent.
        </p>
      )}
    </div>
  );
}
