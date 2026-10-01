import {
  isPermissionGranted,
  requestPermission,
  sendNotification
} from "@tauri-apps/plugin-notification";
import { isTauriRuntime } from "@/lib/desktop-api";
import type { Agent, RuntimeEvent } from "@/types/domain";

export interface DesktopNotification {
  title: string;
  body: string;
}

export type NotificationDelivery = "sent" | "denied" | "unsupported";

const enabledPreferenceKey = "open-bots.notifications.enabled";

export function isNotificationSupported(): boolean {
  return isTauriRuntime();
}

/** Only events that need the user's attention produce a notification. */
export function notificationFor(event: RuntimeEvent, agents: Agent[]): DesktopNotification | null {
  const agentId = event.payload.agentId;
  const agentName = agents.find((agent) => agent.id === agentId)?.name ?? "An agent";
  const text = (key: string) => {
    const value = event.payload[key];
    return typeof value === "string" ? value : "";
  };
  switch (event.eventType) {
    case "approval.requested":
      return {
        title: "Approval needed",
        body: `${agentName} wants to run: ${text("action")}`.trim()
      };
    case "routine.triggered":
      return { title: `${agentName} · routine`, body: text("name") || "A routine was triggered" };
    default:
      return null;
  }
}

/** Notifications are for when the user is elsewhere; a focused window already shows the change. */
export function shouldNotify(enabled: boolean, windowFocused: boolean): boolean {
  return enabled && !windowFocused;
}

export async function deliverNotification(
  notification: DesktopNotification
): Promise<NotificationDelivery> {
  if (!isNotificationSupported()) return "unsupported";
  let granted = await isPermissionGranted();
  if (!granted) granted = (await requestPermission()) === "granted";
  if (!granted) return "denied";
  sendNotification(notification);
  return "sent";
}

export function readNotificationPreference(storage: Pick<Storage, "getItem"> | undefined): boolean {
  try {
    return storage?.getItem(enabledPreferenceKey) !== "false";
  } catch {
    return true;
  }
}

export function writeNotificationPreference(
  storage: Pick<Storage, "setItem"> | undefined,
  enabled: boolean
): void {
  try {
    storage?.setItem(enabledPreferenceKey, String(enabled));
  } catch {
    // Storage can be unavailable; the preference then only lasts for this session.
  }
}
