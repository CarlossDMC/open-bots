import { Bot } from "lucide-react";
import { cn } from "@/lib/utils";
import type { AgentColor, AgentStatus } from "@/types/domain";

const colors: Record<AgentColor, string> = {
  indigo: "from-indigo-400/25 to-indigo-600/5 text-indigo-300 ring-indigo-400/25",
  cyan: "from-cyan-400/25 to-cyan-600/5 text-cyan-300 ring-cyan-400/25",
  emerald: "from-emerald-400/25 to-emerald-600/5 text-emerald-300 ring-emerald-400/25",
  amber: "from-amber-400/25 to-amber-600/5 text-amber-300 ring-amber-400/25",
  rose: "from-rose-400/25 to-rose-600/5 text-rose-300 ring-rose-400/25",
  violet: "from-violet-400/25 to-violet-600/5 text-violet-300 ring-violet-400/25"
};

const statusColors: Record<AgentStatus, string> = {
  idle: "bg-zinc-500",
  working: "bg-blue-400",
  waiting: "bg-amber-400",
  paused: "bg-zinc-500",
  failed: "bg-red-400",
  completed: "bg-emerald-400"
};

const sizes = { sm: "size-9", md: "size-11", lg: "size-16" } as const;
const iconSizes = { sm: 17, md: 20, lg: 28 } as const;

interface AgentAvatarProps {
  color: AgentColor;
  variant: string;
  status: AgentStatus;
  size?: keyof typeof sizes;
}

export function AgentAvatar({ color, variant, status, size = "md" }: AgentAvatarProps) {
  return (
    <div className="relative shrink-0" title={`${variant} avatar, ${status}`}>
      <div
        className={cn(
          "grid place-items-center rounded-lg bg-gradient-to-br ring-1",
          sizes[size],
          colors[color]
        )}
      >
        <Bot size={iconSizes[size]} strokeWidth={1.7} aria-hidden="true" />
      </div>
      <span
        className={cn(
          "absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-zinc-950",
          statusColors[status]
        )}
        aria-label={`Status: ${status}`}
      />
    </div>
  );
}
