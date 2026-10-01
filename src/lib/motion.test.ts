import { describe, expect, it } from "vitest";
import { avatarMotion, avatarScanSegments, fadeUp, iconSwap, scaleIn } from "./motion";
import { maxDuration } from "@/styles/motion-tokens";

const presets = { fadeUp, scaleIn, iconSwap };
const animatable = new Set(["opacity", "x", "y", "scale", "rotate"]);
const targetsOf = (variants: object) =>
  Object.values(variants) as unknown as Record<string, unknown>[];

describe("motion presets", () => {
  it.each(Object.entries(presets))("%s stays within the duration ceiling", (_name, variants) => {
    for (const target of targetsOf(variants)) {
      const transition = target.transition as { duration?: number } | undefined;
      expect((transition?.duration ?? 0) * 1000).toBeLessThanOrEqual(maxDuration);
    }
  });

  it.each(Object.entries(presets))("%s animates only opacity and transforms", (_name, variants) => {
    for (const target of targetsOf(variants)) {
      const properties = Object.keys(target).filter((key) => key !== "transition");
      expect(properties.filter((key) => !animatable.has(key))).toEqual([]);
    }
  });
});

describe("avatar motion", () => {
  it("keeps every blink and scan step within the duration ceiling", () => {
    expect((avatarMotion.blink(4).duration ?? 0) * 1000).toBeLessThanOrEqual(maxDuration);
    const scanStep = ((avatarMotion.scan.duration ?? 0) * 1000) / avatarScanSegments;
    expect(scanStep).toBeLessThanOrEqual(maxDuration);
  });

  it("pauses between repetitions", () => {
    expect(avatarMotion.blink(3).repeatDelay).toBeGreaterThanOrEqual(3);
    expect(avatarMotion.scan.repeatDelay).toBeGreaterThan(1);
  });
});
