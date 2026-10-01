import { BellRing } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Notifications } from "@/hooks/use-notifications";

export function NotificationSettings({ notifications }: { notifications: Notifications }) {
  const { supported, enabled, lastDelivery, error } = notifications;
  return (
    <div className="space-y-3">
      <label className="flex items-center justify-between border-b border-border-subtle py-2.5">
        <span className="text-xs text-foreground-subtle">
          Replies, failures, approvals, and routines
          <span className="block text-2xs text-foreground-faint">
            Shown while Open Bots is in the background
          </span>
        </span>
        <input
          type="checkbox"
          className="accent-foreground"
          checked={enabled}
          disabled={!supported}
          onChange={(event) => notifications.setEnabled(event.target.checked)}
        />
      </label>
      <div className="flex items-center gap-3">
        <Button
          size="sm"
          variant="secondary"
          disabled={!supported}
          onClick={notifications.sendTest}
        >
          <BellRing size={13} />
          Send test notification
        </Button>
        <span className="text-xs text-foreground-subtle" role="status">
          {describeDelivery(supported, lastDelivery, error)}
        </span>
      </div>
    </div>
  );
}

function describeDelivery(
  supported: boolean,
  lastDelivery: Notifications["lastDelivery"],
  error?: string
): string {
  if (!supported) return "Unavailable in browser preview";
  switch (lastDelivery) {
    case "sent":
      return "Sent";
    case "denied":
      return "Blocked by the operating system";
    case "failed":
      return error ?? "Failed";
    default:
      return "";
  }
}
