import { useCallback, useState } from "react";
import { browserStorage } from "@/lib/browser-storage";
import {
  deliverNotification,
  isNotificationSupported,
  notificationFor,
  readNotificationPreference,
  shouldNotify,
  writeNotificationPreference,
  type NotificationDelivery
} from "@/lib/notifications";
import { describeError } from "@/lib/utils";
import type { Agent, RuntimeEvent } from "@/types/domain";

export interface Notifications {
  supported: boolean;
  enabled: boolean;
  setEnabled: (enabled: boolean) => void;
  /** Result of the most recent delivery attempt, for the settings page. */
  lastDelivery?: NotificationDelivery | "failed";
  error?: string;
  notifyFor: (event: RuntimeEvent, agents: Agent[]) => void;
  sendTest: () => void;
}

export function useNotifications(): Notifications {
  const supported = isNotificationSupported();
  const [enabled, setEnabledState] = useState(() => readNotificationPreference(browserStorage()));
  const [lastDelivery, setLastDelivery] = useState<Notifications["lastDelivery"]>();
  const [error, setError] = useState<string>();

  const deliver = useCallback((notification: { title: string; body: string }) => {
    deliverNotification(notification).then(
      (result) => {
        setLastDelivery(result);
        setError(undefined);
      },
      (caught: unknown) => {
        setLastDelivery("failed");
        setError(describeError(caught, "The notification could not be shown."));
      }
    );
  }, []);

  const setEnabled = useCallback((value: boolean) => {
    setEnabledState(value);
    writeNotificationPreference(browserStorage(), value);
  }, []);

  const notifyFor = useCallback(
    (event: RuntimeEvent, agents: Agent[]) => {
      if (!shouldNotify(enabled, document.hasFocus())) return;
      const notification = notificationFor(event, agents);
      if (notification) deliver(notification);
    },
    [deliver, enabled]
  );

  const sendTest = useCallback(() => {
    deliver({ title: "Open Bots", body: "Notifications are working." });
  }, [deliver]);

  return { supported, enabled, setEnabled, lastDelivery, error, notifyFor, sendTest };
}
