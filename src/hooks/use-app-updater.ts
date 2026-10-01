import { useCallback, useEffect, useRef, useState } from "react";
import { browserStorage } from "@/lib/browser-storage";
import {
  checkForAppUpdate,
  describeUpdateError,
  isUpdaterSupported,
  readAutoCheckPreference,
  restartApp,
  writeAutoCheckPreference,
  type UpdateState
} from "@/lib/updater";

export interface AppUpdater {
  state: UpdateState;
  checkOnLaunch: boolean;
  setCheckOnLaunch: (enabled: boolean) => void;
  check: () => Promise<void>;
  install: () => Promise<void>;
  dismiss: () => void;
}

export function useAppUpdater(): AppUpdater {
  const supported = isUpdaterSupported();
  const [state, setState] = useState<UpdateState>(
    supported ? { status: "idle" } : { status: "unsupported" }
  );
  const [checkOnLaunch, setCheckOnLaunchState] = useState(() =>
    readAutoCheckPreference(browserStorage())
  );
  const busy = useRef(false);

  const check = useCallback(async () => {
    if (!supported || busy.current) return;
    busy.current = true;
    setState({ status: "checking" });
    try {
      const update = await checkForAppUpdate();
      setState(
        update
          ? { status: "available", update }
          : { status: "up-to-date", checkedAt: new Date().toISOString() }
      );
    } catch (error) {
      setState({ status: "error", message: describeUpdateError(error) });
    } finally {
      busy.current = false;
    }
  }, [supported]);

  const install = useCallback(async () => {
    if (busy.current) return;
    const update = "update" in state ? state.update : undefined;
    if (!update) return;
    busy.current = true;
    setState({ status: "downloading", update, progress: { downloadedBytes: 0 } });
    try {
      await update.install((progress) => setState({ status: "downloading", update, progress }));
      setState({ status: "restarting", update });
      await restartApp();
    } catch (error) {
      setState({ status: "error", message: describeUpdateError(error), update });
    } finally {
      busy.current = false;
    }
  }, [state]);

  const dismiss = useCallback(() => {
    setState((current) =>
      current.status === "available" || current.status === "error" ? { status: "idle" } : current
    );
  }, []);

  const setCheckOnLaunch = useCallback((enabled: boolean) => {
    setCheckOnLaunchState(enabled);
    writeAutoCheckPreference(browserStorage(), enabled);
  }, []);

  const launchCheckDone = useRef(false);
  useEffect(() => {
    if (launchCheckDone.current || !checkOnLaunch) return;
    launchCheckDone.current = true;
    void check();
  }, [check, checkOnLaunch]);

  return { state, checkOnLaunch, setCheckOnLaunch, check, install, dismiss };
}
