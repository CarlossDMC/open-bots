import { m } from "motion/react";
import { cn } from "@/lib/utils";
import { avatarMotion } from "@/lib/motion";
import type { AgentColor, AgentStatus } from "@/types/domain";
import {
  blinkDelayFor,
  expressionFor,
  eyeShapeFor,
  type AvatarExpression,
  type EyeShape
} from "./avatar-expression";

const colors: Record<AgentColor, string> = {
  indigo: "bg-identity-indigo",
  cyan: "bg-identity-cyan",
  emerald: "bg-identity-emerald",
  amber: "bg-identity-amber",
  rose: "bg-identity-rose",
  violet: "bg-identity-violet"
};

const statusColors: Record<AgentStatus, string> = {
  idle: "bg-status-neutral",
  working: "bg-status-running",
  waiting: "bg-status-waiting",
  paused: "bg-status-neutral",
  failed: "bg-status-danger",
  completed: "bg-status-success"
};

const sizes = {
  sm: "size-9 rounded-lg",
  md: "size-11 rounded-lg",
  lg: "size-16 rounded-xl"
} as const;

// Eye geometry in a 24x24 face, centered on each eye.
const eyes: Record<EyeShape, { width: number; height: number }> = {
  dot: { width: 4, height: 4.6 },
  pill: { width: 3, height: 6.4 },
  wide: { width: 6, height: 3.8 }
};
const eyeCenters = [8, 16] as const;
const eyeY = 11;

const gaze: Record<AvatarExpression, { x: number | number[]; y: number; scaleY: number }> = {
  neutral: { x: 0, y: 0, scaleY: 1 },
  scanning: { x: [0, -1.6, 1.6, 0], y: 0, scaleY: 1 },
  "looking-up": { x: 0, y: -1.6, scaleY: 1 },
  closed: { x: 0, y: 0.6, scaleY: 0.2 },
  crossed: { x: 0, y: 0, scaleY: 1 }
};

const originCenter = { transformBox: "fill-box", transformOrigin: "center" } as const;

interface AgentAvatarProps {
  color: AgentColor;
  variant: string;
  status: AgentStatus;
  size?: keyof typeof sizes;
  /** Stable value, such as the agent id, that desynchronizes blinking across a list. */
  seed?: string;
}

export function AgentAvatar({ color, variant, status, size = "md", seed }: AgentAvatarProps) {
  const expression = expressionFor(status);
  const shape = eyeShapeFor(variant);
  const target = gaze[expression];
  const blinks = expression !== "closed" && expression !== "crossed";

  return (
    <m.div
      className="relative shrink-0"
      title={`${variant} avatar, ${status}`}
      initial="rest"
      animate="rest"
      whileHover="hover"
    >
      <div
        className={cn(
          "grid place-items-center ring-1 ring-inset ring-foreground/10",
          sizes[size],
          colors[color]
        )}
      >
        <svg viewBox="0 0 24 24" className="size-full" aria-hidden="true">
          {expression === "crossed" ? (
            <g className="stroke-avatar-eye" strokeWidth={1.8} strokeLinecap="round">
              {eyeCenters.map((x) => (
                <path key={x} d={`M${x - 1.8} ${eyeY - 1.8}l3.6 3.6m0 -3.6l-3.6 3.6`} />
              ))}
            </g>
          ) : (
            <m.g
              style={originCenter}
              variants={{ rest: { scale: 1 }, hover: { scale: 1.15 } }}
              transition={avatarMotion.gaze}
            >
              <m.g
                style={originCenter}
                initial={false}
                animate={{ x: target.x, y: target.y, scaleY: target.scaleY }}
                transition={expression === "scanning" ? avatarMotion.scan : avatarMotion.gaze}
              >
                <m.g
                  style={originCenter}
                  animate={blinks ? { scaleY: [1, 0.1, 1] } : { scaleY: 1 }}
                  transition={
                    blinks ? avatarMotion.blink(blinkDelayFor(seed ?? variant)) : avatarMotion.gaze
                  }
                  className="fill-avatar-eye"
                >
                  {eyeCenters.map((x) => (
                    <rect
                      key={x}
                      x={x - eyes[shape].width / 2}
                      y={eyeY - eyes[shape].height / 2}
                      width={eyes[shape].width}
                      height={eyes[shape].height}
                      rx={Math.min(eyes[shape].width, eyes[shape].height) / 2}
                    />
                  ))}
                </m.g>
              </m.g>
            </m.g>
          )}
        </svg>
      </div>
      <span
        className={cn(
          "absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-background",
          statusColors[status]
        )}
        aria-label={`Status: ${status}`}
      />
    </m.div>
  );
}
