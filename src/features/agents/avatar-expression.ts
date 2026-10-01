import type { AgentStatus } from "@/types/domain";

export type EyeShape = "dot" | "pill" | "wide";
export type AvatarExpression = "neutral" | "scanning" | "looking-up" | "closed" | "crossed";

const eyeShapes: readonly EyeShape[] = ["dot", "pill", "wide"];
const knownVariants: Record<string, EyeShape> = { orbital: "dot", signal: "pill", halo: "wide" };

/** Stable, non-cryptographic hash used to spread unknown variants and blink timing. */
export function variantSeed(variant: string): number {
  let hash = 0;
  for (const character of variant) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  return hash;
}

export function eyeShapeFor(variant: string): EyeShape {
  return knownVariants[variant] ?? eyeShapes[variantSeed(variant) % eyeShapes.length];
}

const expressions: Record<AgentStatus, AvatarExpression> = {
  idle: "neutral",
  completed: "neutral",
  working: "scanning",
  waiting: "looking-up",
  paused: "closed",
  failed: "crossed"
};

export function expressionFor(status: AgentStatus): AvatarExpression {
  return expressions[status];
}

/** Seconds between blinks, between 3 and 6, so a list of avatars does not blink in sync. */
export function blinkDelayFor(seed: string): number {
  return 3 + (variantSeed(seed) % 31) / 10;
}
