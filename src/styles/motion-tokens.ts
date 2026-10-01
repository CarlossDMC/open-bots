// Motion timing shared by Tailwind utilities (tailwind.config.ts) and Motion presets (src/lib/motion.ts).
export type CubicBezier = [number, number, number, number];

/** Durations in milliseconds. `slow` is the ceiling for any UI animation. */
export const durations = { fast: 120, base: 180, slow: 240 } as const;
export const maxDuration = durations.slow;

export const easings: { standard: CubicBezier; exit: CubicBezier } = {
  standard: [0.2, 0, 0, 1],
  exit: [0.4, 0, 1, 1]
};

export function cubicBezier([x1, y1, x2, y2]: CubicBezier): string {
  return `cubic-bezier(${x1}, ${y1}, ${x2}, ${y2})`;
}
