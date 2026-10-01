import { Import, Loader2 } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { useMcpCatalog } from "@/hooks/use-mcp-catalog";
import { discoverMcpServers } from "@/lib/desktop-api";
import { mergeFound, type ReviewServer } from "@/lib/mcp-catalog";
import { supportsConfiguredMcpServers } from "@/lib/models";
import { describeError } from "@/lib/utils";
import type { McpServerStatus, ProviderSummary } from "@/types/domain";

const statusLabels: Record<McpServerStatus | "missing", string> = {
  connected: "Connected",
  "needs-authentication": "Needs authentication in the provider",
  failed: "Failed to connect",
  "pending-approval": "Pending approval in the provider",
  unknown: "Status unknown",
  missing: "No longer in the provider configuration"
};

interface Review {
  providerId: string;
  servers: ReviewServer[];
  chosen: string[];
}

/** The global MCP server catalog: import names from each provider, then choose which stay. */
export function McpCatalogSettings({ providers }: { providers: ProviderSummary[] }) {
  const catalog = useMcpCatalog();
  const [discovering, setDiscovering] = useState<string>();
  const [review, setReview] = useState<Review>();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const supported = providers.filter(supportsConfiguredMcpServers);

  async function startImport(providerId: string) {
    setDiscovering(providerId);
    setError(undefined);
    try {
      const found = await discoverMcpServers(providerId);
      const current = catalog.entries
        .filter((entry) => entry.providerId === providerId)
        .map((entry) => entry.name);
      setReview({
        providerId,
        servers: mergeFound(found, current),
        chosen: current
      });
    } catch (caught) {
      setError(describeError(caught, "The MCP servers could not be read."));
    } finally {
      setDiscovering(undefined);
    }
  }

  async function saveReview() {
    if (!review) return;
    setSaving(true);
    if (await catalog.save(review.providerId, review.chosen)) setReview(undefined);
    setSaving(false);
  }

  if (supported.length === 0) {
    return (
      <p className="text-xs text-foreground-subtle">
        No installed provider can share its MCP servers with Open Bots.
      </p>
    );
  }

  return (
    <div className="space-y-3">
      {supported.map((provider) => {
        const names = catalog.entries
          .filter((entry) => entry.providerId === provider.id)
          .map((entry) => entry.name);
        const reviewing = review?.providerId === provider.id ? review : undefined;
        return (
          <div key={provider.id} className="rounded-md border border-border-subtle bg-card p-3">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <p className="text-sm text-foreground">{provider.name}</p>
                <p className="mt-0.5 truncate text-xs text-foreground-subtle">
                  {catalog.loading
                    ? "Loading…"
                    : names.length > 0
                      ? names.join(", ")
                      : "No servers in the catalog"}
                </p>
              </div>
              <Button
                size="sm"
                variant="secondary"
                disabled={Boolean(discovering) || saving}
                onClick={() => void startImport(provider.id)}
              >
                {discovering === provider.id ? (
                  <Loader2 size={14} className="animate-spin" />
                ) : (
                  <Import size={14} />
                )}
                {discovering === provider.id ? "Checking servers…" : `Import from ${provider.name}`}
              </Button>
            </div>
            {reviewing && (
              <ReviewList
                review={reviewing}
                saving={saving}
                onChange={setReview}
                onCancel={() => setReview(undefined)}
                onSave={() => void saveReview()}
              />
            )}
          </div>
        );
      })}
      <p className="text-xs leading-5 text-foreground-faint">
        Only server names are imported; their configuration and credentials stay with the provider.
        Agents of that provider can then select them. Every tool of a selected server runs without
        approval, including tools that change data in external services.
      </p>
      {(error ?? catalog.error) && (
        <p className="text-xs text-danger-foreground" role="alert">
          {error ?? catalog.error}
        </p>
      )}
    </div>
  );
}

function ReviewList({
  review,
  saving,
  onChange,
  onCancel,
  onSave
}: {
  review: Review;
  saving: boolean;
  onChange: (review: Review) => void;
  onCancel: () => void;
  onSave: () => void;
}) {
  function toggle(name: string, checked: boolean) {
    onChange({
      ...review,
      chosen: checked
        ? [...review.chosen, name]
        : review.chosen.filter((candidate) => candidate !== name)
    });
  }

  return (
    <div className="mt-3 border-t border-border-subtle pt-3">
      {review.servers.length === 0 ? (
        <p className="text-xs text-foreground-subtle">
          The provider has no MCP servers configured.
        </p>
      ) : (
        <ul className="space-y-1.5" aria-label="Servers found">
          {review.servers.map((server) => {
            const id = `catalog-${review.providerId}-${server.name.replace(/\W+/g, "-")}`;
            return (
              <li key={server.name} className="flex items-center gap-2">
                <Checkbox
                  id={id}
                  checked={review.chosen.includes(server.name)}
                  disabled={saving}
                  onCheckedChange={(checked) => toggle(server.name, checked)}
                />
                <label htmlFor={id} className="text-sm text-foreground-secondary">
                  {server.name}
                </label>
                <span className="ml-auto text-xs text-foreground-faint">
                  {statusLabels[server.status]}
                </span>
              </li>
            );
          })}
        </ul>
      )}
      <div className="mt-3 flex justify-end gap-2">
        <Button size="sm" variant="ghost" disabled={saving} onClick={onCancel}>
          Cancel
        </Button>
        <Button size="sm" disabled={saving} onClick={onSave}>
          {saving ? "Saving…" : "Save catalog"}
        </Button>
      </div>
    </div>
  );
}
