import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";
import { isTauriRuntime } from "@/lib/desktop-api";

export interface DownloadProgress {
  downloadedBytes: number;
  totalBytes?: number;
}

export interface AvailableUpdate {
  version: string;
  currentVersion: string;
  notes?: string;
  install: (onProgress: (progress: DownloadProgress) => void) => Promise<void>;
}

export type UpdateState =
  | { status: "unsupported" }
  | { status: "idle" }
  | { status: "checking" }
  | { status: "up-to-date"; checkedAt: string }
  | { status: "available"; update: AvailableUpdate }
  | { status: "downloading"; update: AvailableUpdate; progress: DownloadProgress }
  | { status: "restarting"; update: AvailableUpdate }
  | { status: "error"; message: string; update?: AvailableUpdate };

const autoCheckPreferenceKey = "open-bots.updates.check-on-launch";

export function isUpdaterSupported(): boolean {
  return isTauriRuntime();
}

/** Queries the configured release endpoint. Resolves to `null` when the installed version is current. */
export async function checkForAppUpdate(): Promise<AvailableUpdate | null> {
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    currentVersion: update.currentVersion,
    notes: update.body?.trim() || undefined,
    async install(onProgress) {
      let progress: DownloadProgress = { downloadedBytes: 0 };
      await update.downloadAndInstall((event) => {
        progress = applyDownloadEvent(progress, event);
        onProgress(progress);
      });
    }
  };
}

export async function restartApp(): Promise<void> {
  await relaunch();
}

export function applyDownloadEvent(
  progress: DownloadProgress,
  event: DownloadEvent
): DownloadProgress {
  switch (event.event) {
    case "Started":
      return { downloadedBytes: 0, totalBytes: event.data.contentLength };
    case "Progress":
      return { ...progress, downloadedBytes: progress.downloadedBytes + event.data.chunkLength };
    case "Finished":
      return progress.totalBytes === undefined
        ? progress
        : { ...progress, downloadedBytes: progress.totalBytes };
  }
}

/** Returns a 0-100 percentage, or `undefined` when the server did not report a content length. */
export function downloadPercentage(progress: DownloadProgress): number | undefined {
  if (!progress.totalBytes) return undefined;
  return Math.min(100, Math.round((progress.downloadedBytes / progress.totalBytes) * 100));
}

export function describeUpdateError(error: unknown): string {
  const detail = error instanceof Error ? error.message : typeof error === "string" ? error : "";
  return detail.trim() || "The update service could not be reached.";
}

export function readAutoCheckPreference(storage: Pick<Storage, "getItem"> | undefined): boolean {
  try {
    return storage?.getItem(autoCheckPreferenceKey) !== "false";
  } catch {
    return true;
  }
}

export function writeAutoCheckPreference(
  storage: Pick<Storage, "setItem"> | undefined,
  enabled: boolean
): void {
  try {
    storage?.setItem(autoCheckPreferenceKey, String(enabled));
  } catch {
    // Storage can be unavailable; the preference then only lasts for this session.
  }
}
