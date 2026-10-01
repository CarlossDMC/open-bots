import { describe, expect, it } from "vitest";
import { blinkDelayFor, expressionFor, eyeShapeFor } from "./avatar-expression";

describe("expressionFor", () => {
  it.each([
    ["idle", "neutral"],
    ["completed", "neutral"],
    ["working", "scanning"],
    ["waiting", "looking-up"],
    ["paused", "closed"],
    ["failed", "crossed"]
  ] as const)("maps %s to %s", (status, expression) => {
    expect(expressionFor(status)).toBe(expression);
  });
});

describe("eyeShapeFor", () => {
  it("maps known variants to their eye style", () => {
    expect(eyeShapeFor("orbital")).toBe("dot");
    expect(eyeShapeFor("signal")).toBe("pill");
    expect(eyeShapeFor("halo")).toBe("wide");
  });

  it("gives unknown variants a deterministic eye style", () => {
    expect(eyeShapeFor("custom-variant")).toBe(eyeShapeFor("custom-variant"));
    expect(["dot", "pill", "wide"]).toContain(eyeShapeFor("custom-variant"));
  });
});

describe("blinkDelayFor", () => {
  it("stays between three and six seconds", () => {
    for (const seed of ["a", "agent-1", "agent-2", "a much longer identifier"]) {
      const delay = blinkDelayFor(seed);
      expect(delay).toBeGreaterThanOrEqual(3);
      expect(delay).toBeLessThanOrEqual(6);
    }
  });
});
