import { Info, MessageSquare, Mic, Plus, Sparkles } from "lucide-react";
import { useState } from "react";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { AgentDetails } from "@/features/agents/agent-details";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/ui/status-badge";
import { cn, formatConversationTime } from "@/lib/utils";
import type { Agent } from "@/types/domain";

export function AgentConversation({ agent }: { agent: Agent }) {
  const [showDetails, setShowDetails] = useState(false);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex h-12 shrink-0 items-center gap-2.5 border-b border-border-subtle px-4">
        <AgentAvatar
          color={agent.identityColor}
          variant={agent.avatarVariant}
          seed={agent.id}
          status={agent.status}
          size="sm"
        />
        <h1 className="truncate text-sm font-medium text-foreground">{agent.name}</h1>
        <StatusBadge status={agent.status} className="text-2xs" />
        <button
          type="button"
          onClick={() => setShowDetails((current) => !current)}
          aria-label={showDetails ? "Hide agent details" : "Show agent details"}
          aria-pressed={showDetails}
          title="Agent details"
          className={cn(
            "ml-auto grid size-7 place-items-center rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            showDetails
              ? "bg-accent text-foreground"
              : "text-foreground-subtle hover:bg-muted hover:text-foreground"
          )}
        >
          <Info size={15} strokeWidth={1.8} />
        </button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {showDetails ? (
          <div className="mx-auto max-w-5xl px-8 py-8">
            <AgentDetails agent={agent} />
          </div>
        ) : (
          <ol className="mx-auto max-w-3xl space-y-4 px-8 py-8" aria-label="Conversation">
            <TimelineEntry
              icon={Sparkles}
              text={`Created as ${agent.role}`}
              time={formatConversationTime(agent.createdAt)}
            />
            {agent.currentTask && (
              <TimelineEntry
                icon={MessageSquare}
                text={agent.currentTask}
                time={formatConversationTime(agent.updatedAt)}
              />
            )}
            <li className="flex items-center gap-3 pl-1">
              <StatusBadge status={agent.status} className="text-foreground-subtle" />
            </li>
          </ol>
        )}
      </div>
      <Composer agentName={agent.name} />
    </div>
  );
}

function TimelineEntry({
  icon: Icon,
  text,
  time
}: {
  icon: typeof Sparkles;
  text: string;
  time: string;
}) {
  return (
    <li className="flex items-start gap-3 text-sm">
      <Icon size={14} className="mt-0.5 shrink-0 text-foreground-faint" aria-hidden="true" />
      <span className="min-w-0 flex-1 text-foreground-muted">{text}</span>
      <span className="shrink-0 text-2xs text-foreground-faint">{time}</span>
    </li>
  );
}

function Composer({ agentName }: { agentName: string }) {
  return (
    <div className="shrink-0 px-5 pb-3">
      <div className="mx-auto max-w-3xl">
        <div className="flex h-11 items-center gap-2 rounded-full border border-border bg-card px-1.5 opacity-80">
          <span className="grid size-8 place-items-center rounded-full bg-muted text-foreground-faint">
            <Plus size={16} aria-hidden="true" />
          </span>
          <input
            disabled
            aria-label={`Message ${agentName}`}
            aria-describedby="composer-status"
            placeholder={`Message ${agentName}`}
            className="h-full min-w-0 flex-1 cursor-not-allowed bg-transparent text-sm text-foreground outline-none placeholder:text-foreground-faint"
          />
          <span className="grid size-8 place-items-center rounded-full bg-muted text-foreground-faint">
            <Mic size={15} aria-hidden="true" />
          </span>
        </div>
        <p id="composer-status" className="mt-1.5 text-center text-2xs text-foreground-faint">
          Messaging is not implemented yet.
        </p>
      </div>
    </div>
  );
}

export function NoConversation({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="grid flex-1 place-items-center">
      <div className="text-center">
        <MessageSquare className="mx-auto mb-3 text-foreground-faint" size={22} />
        <p className="text-sm text-foreground-secondary">No agent selected</p>
        <p className="mt-1 text-xs text-foreground-faint">
          Pick an agent from the list or create a new one.
        </p>
        <Button size="sm" variant="secondary" className="mt-4" onClick={onCreate}>
          <Plus size={14} /> New agent
        </Button>
      </div>
    </div>
  );
}
