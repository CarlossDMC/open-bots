import { Download, RotateCw, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { AppUpdater } from "@/hooks/use-app-updater";
import { downloadPercentage } from "@/lib/updater";

/** Shown only for update activity the user can act on; silent otherwise. */
export function UpdateBanner({ updater }: { updater: AppUpdater }) {
  const { state } = updater;
  if (state.status === "available") {
    return (
      <Banner onDismiss={updater.dismiss}>
        <span>
          Open Bots {state.update.version} is available. You are running{" "}
          {state.update.currentVersion}.
        </span>
        <Button size="sm" variant="secondary" onClick={() => void updater.install()}>
          <Download size={13} /> Install and restart
        </Button>
      </Banner>
    );
  }
  if (state.status === "downloading") {
    const percentage = downloadPercentage(state.progress);
    return (
      <Banner>
        <span role="status">
          Downloading Open Bots {state.update.version}
          {percentage === undefined ? "…" : ` — ${percentage}%`}
        </span>
      </Banner>
    );
  }
  if (state.status === "restarting") {
    return (
      <Banner>
        <span role="status">Update installed. Restarting…</span>
      </Banner>
    );
  }
  if (state.status === "error" && state.update) {
    return (
      <Banner tone="error" onDismiss={updater.dismiss}>
        <span>Update failed: {state.message}</span>
        <Button size="sm" variant="secondary" onClick={() => void updater.install()}>
          <RotateCw size={13} /> Retry
        </Button>
      </Banner>
    );
  }
  return null;
}

function Banner({
  children,
  tone = "default",
  onDismiss
}: {
  children: React.ReactNode;
  tone?: "default" | "error";
  onDismiss?: () => void;
}) {
  return (
    <div
      className={
        tone === "error"
          ? "flex items-center gap-3 border-b border-danger-border bg-danger-muted px-8 py-2 text-xs text-danger-foreground"
          : "flex items-center gap-3 border-b border-border/80 bg-card px-8 py-2 text-xs text-foreground-secondary"
      }
    >
      <div className="flex flex-1 items-center gap-3">{children}</div>
      {onDismiss && (
        <button
          className="text-foreground-subtle hover:text-foreground"
          aria-label="Dismiss update notice"
          onClick={onDismiss}
        >
          <X size={14} />
        </button>
      )}
    </div>
  );
}
