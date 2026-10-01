import type { Transition, Variants } from "motion/react";
import { durations, easings } from "@/styles/motion-tokens";

// Motion conventions are documented in docs/design-system.md#motion.
const seconds = (milliseconds: number) => milliseconds / 1000;

export const transitions = {
  enter: { duration: seconds(durations.base), ease: easings.standard },
  exit: { duration: seconds(durations.fast), ease: easings.exit }
} satisfies Record<string, Transition>;

/** Content appearing in place: pages, panels, banners. */
export const fadeUp: Variants = {
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0, transition: transitions.enter },
  exit: { opacity: 0, y: 4, transition: transitions.exit }
};

/** Overlays anchored to the viewport: dialogs and the command palette. */
export const scaleIn: Variants = {
  initial: { opacity: 0, scale: 0.98 },
  animate: { opacity: 1, scale: 1, transition: transitions.enter },
  exit: { opacity: 0, scale: 0.98, transition: transitions.exit }
};

/** Two icons replacing each other in the same slot, such as the theme toggle. */
export const iconSwap: Variants = {
  initial: { opacity: 0, rotate: -90, scale: 0.6 },
  animate: { opacity: 1, rotate: 0, scale: 1, transition: transitions.enter },
  exit: { opacity: 0, rotate: 90, scale: 0.6, transition: transitions.exit }
};

/**
 * Ambient agent avatar motion. Each step stays within the duration ceiling and repeats after a
 * pause; the movement reflects runtime status only. Reduced motion disables it through MotionConfig.
 */
export const avatarMotion = {
  blink: (repeatDelay: number): Transition => ({
    duration: seconds(durations.fast + 40),
    ease: easings.standard,
    repeat: Infinity,
    repeatDelay
  }),
  scan: {
    duration: seconds(durations.slow * 3),
    ease: easings.standard,
    repeat: Infinity,
    repeatDelay: 1.4
  },
  gaze: transitions.enter
} satisfies Record<string, Transition | ((repeatDelay: number) => Transition)>;

/** Number of keyframe segments in avatarMotion.scan. */
export const avatarScanSegments = 3;
