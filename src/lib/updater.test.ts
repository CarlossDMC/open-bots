import { describe, expect, it } from "vitest";
import {
  applyDownloadEvent,
  describeUpdateError,
  downloadPercentage,
  readAutoCheckPreference,
  writeAutoCheckPreference
} from "./updater";

describe("applyDownloadEvent", () => {
  it("accumulates chunks against the reported content length", () => {
    let progress = applyDownloadEvent(
      { downloadedBytes: 0 },
      { event: "Started", data: { contentLength: 200 } }
    );
    progress = applyDownloadEvent(progress, { event: "Progress", data: { chunkLength: 50 } });
    progress = applyDownloadEvent(progress, { event: "Progress", data: { chunkLength: 25 } });
    expect(progress).toEqual({ downloadedBytes: 75, totalBytes: 200 });
    expect(downloadPercentage(progress)).toBe(38);
    expect(applyDownloadEvent(progress, { event: "Finished" })).toEqual({
      downloadedBytes: 200,
      totalBytes: 200
    });
  });

  it("reports no percentage when the content length is unknown", () => {
    const progress = applyDownloadEvent(
      applyDownloadEvent({ downloadedBytes: 0 }, { event: "Started", data: {} }),
      { event: "Progress", data: { chunkLength: 10 } }
    );
    expect(downloadPercentage(progress)).toBeUndefined();
    expect(applyDownloadEvent(progress, { event: "Finished" })).toEqual(progress);
  });
});

describe("describeUpdateError", () => {
  it("keeps actionable messages and falls back for empty errors", () => {
    expect(describeUpdateError(new Error("signature mismatch"))).toBe("signature mismatch");
    expect(describeUpdateError("network down")).toBe("network down");
    expect(describeUpdateError({})).toBe("The update service could not be reached.");
  });
});

describe("auto-check preference", () => {
  it("defaults to enabled and round-trips an explicit choice", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => void values.set(key, value)
    };
    expect(readAutoCheckPreference(storage)).toBe(true);
    writeAutoCheckPreference(storage, false);
    expect(readAutoCheckPreference(storage)).toBe(false);
  });

  it("falls back to enabled when storage throws", () => {
    const storage = {
      getItem: () => {
        throw new Error("blocked");
      }
    };
    expect(readAutoCheckPreference(storage)).toBe(true);
    expect(readAutoCheckPreference(undefined)).toBe(true);
  });
});
