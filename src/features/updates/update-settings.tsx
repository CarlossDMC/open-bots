import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type { AppUpdater } from "@/hooks/use-app-updater";
import { downloadPercentage, type UpdateState } from "@/lib/updater";
import { formatRelativeTime } from "@/lib/utils";

export function UpdateSettings({ updater }: { updater: AppUpdater }) {
  const { state } = updater;
  const unsupported = state.status === "unsupported";
  const busy =
    state.status === "checking" || state.status === "downloading" || state.status === "restarting";
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between border-b border-border-subtle py-2.5">
        <span className="text-xs text-foreground-subtle">Status</span>
        <span className="text-xs text-foreground-secondary" role="status">
          {describeState(state)}
        </span>
      </div>
      <label className="flex items-center justify-between border-b border-border-subtle py-2.5">
        <span className="text-xs text-foreground-subtle">Check on launch</span>
        <Checkbox
          checked={updater.checkOnLaunch}
          disabled={unsupported}
          onCheckedChange={updater.setCheckOnLaunch}
        />
      </label>
      <div className="flex gap-2">
        <Button
          size="sm"
          variant="secondary"
          disabled={unsupported || busy}
          onClick={() => void updater.check()}
        >
          <RefreshCw size={13} className={state.status === "checking" ? "animate-spin" : ""} />
          Check for updates
        </Button>
        {state.status === "available" && (
          <Button size="sm" onClick={() => void updater.install()}>
            Install {state.update.version} and restart
          </Button>
        )}
      </div>
    </div>
  );
}

function describeState(state: UpdateState): string {
  switch (state.status) {
    case "unsupported":
      return "Unavailable in browser preview";
    case "idle":
      return "Not checked";
    case "checking":
      return "Checking…";
    case "up-to-date":
      return `Up to date (checked ${formatRelativeTime(state.checkedAt)})`;
    case "available":
      return `Version ${state.update.version} available`;
    case "downloading": {
      const percentage = downloadPercentage(state.progress);
      return percentage === undefined ? "Downloading…" : `Downloading — ${percentage}%`;
    }
    case "restarting":
      return "Restarting…";
    case "error":
      return `Failed: ${state.message}`;
  }
}
