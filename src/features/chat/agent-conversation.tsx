import { AnimatePresence, m, useReducedMotion } from "motion/react";
import {
  ArrowUp,
  Info,
  Loader2,
  MessageSquare,
  Plus,
  Sparkles,
  Square,
  Terminal
} from "lucide-react";
import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { AgentDetails } from "@/features/agents/agent-details";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/ui/status-badge";
import { useConversation } from "@/hooks/use-conversation";
import { useFreshIds } from "@/hooks/use-fresh-ids";
import { isTauriRuntime, messagingUnavailableMessage } from "@/lib/desktop-api";
import { describeModel } from "@/lib/models";
import {
  fadeUp,
  iconSwap,
  messageIn,
  transitions,
  userMessageIn,
  workingDotMotion
} from "@/lib/motion";
import { cn, formatConversationTime } from "@/lib/utils";
import type { Agent, ConversationMessage, ProviderSummary } from "@/types/domain";

export function AgentConversation({
  agent,
  provider,
  onAgentUpdated
}: {
  agent: Agent;
  provider?: ProviderSummary;
  onAgentUpdated?: (agent: Agent) => void;
}) {
  const [showDetails, setShowDetails] = useState(false);
  const conversation = useConversation(agent.id);
  const working = agent.status === "working";
  const unavailableReason = messagingUnavailableReason(provider);
  const endRef = useRef<HTMLLIElement>(null);
  const reduceMotion = useReducedMotion();
  const freshIds = useFreshIds(
    conversation.messages.map((message) => message.id),
    !conversation.loading
  );
  const hasFreshContent = freshIds.size > 0 || working;

  useEffect(() => {
    // Jump straight to the end for the initial history; glide for new arrivals.
    const behavior = hasFreshContent && !reduceMotion ? "smooth" : "auto";
    endRef.current?.scrollIntoView?.({ block: "end", behavior });
  }, [
    conversation.messages.length,
    conversation.currentAction,
    working,
    showDetails,
    hasFreshContent,
    reduceMotion
  ]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex h-14 shrink-0 items-center gap-3 border-b border-border-subtle px-5">
        <AgentAvatar
          color={agent.identityColor}
          variant={agent.avatarVariant}
          seed={agent.id}
          status={agent.status}
          size="sm"
        />
        <div className="min-w-0">
          <h1 className="truncate text-sm font-medium leading-5 text-foreground">{agent.name}</h1>
          <div className="flex min-w-0 items-center gap-1.5 text-xs-plus leading-4">
            <AnimatePresence mode="wait" initial={false}>
              <m.span
                key={agent.status}
                variants={fadeUp}
                initial="initial"
                animate="animate"
                exit="exit"
              >
                <StatusBadge status={agent.status} className="text-xs-plus" />
              </m.span>
            </AnimatePresence>
            {agent.model && (
              <>
                <span className="text-foreground-faint" aria-hidden="true">
                  ·
                </span>
                <span className="truncate text-foreground-faint" title="Model">
                  {describeModel(agent)}
                </span>
              </>
            )}
          </div>
        </div>
        <button
          type="button"
          onClick={() => setShowDetails((current) => !current)}
          aria-label={showDetails ? "Hide agent details" : "Show agent details"}
          aria-pressed={showDetails}
          title="Agent details"
          className={cn(
            "ml-auto grid size-8 place-items-center rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            showDetails
              ? "bg-accent text-foreground"
              : "text-foreground-subtle hover:bg-muted hover:text-foreground"
          )}
        >
          <Info size={16} strokeWidth={1.8} />
        </button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {showDetails ? (
          <div className="mx-auto max-w-5xl px-8 py-8">
            <AgentDetails agent={agent} provider={provider} onAgentUpdated={onAgentUpdated} />
          </div>
        ) : (
          <ol className="mx-auto max-w-3xl space-y-5 px-8 py-8" aria-label="Conversation">
            <TimelineEntry
              icon={Sparkles}
              text={`Created as ${agent.role}`}
              time={formatConversationTime(agent.createdAt)}
            />
            {conversation.messages.map((message) => (
              <MessageEntry
                key={message.id}
                message={message}
                agentName={agent.name}
                fresh={freshIds.has(message.id)}
              />
            ))}
            <AnimatePresence initial={false}>
              {working ? (
                <WorkingIndicator
                  key="working"
                  agentName={agent.name}
                  currentAction={conversation.currentAction}
                />
              ) : null}
            </AnimatePresence>
            <li ref={endRef} aria-hidden="true" />
          </ol>
        )}
      </div>
      <Composer
        agentName={agent.name}
        working={working}
        unavailableReason={unavailableReason}
        error={conversation.error}
        onSend={conversation.send}
        onCancel={() => void conversation.cancel()}
      />
    </div>
  );
}

function messagingUnavailableReason(provider?: ProviderSummary): string | undefined {
  if (!isTauriRuntime()) return messagingUnavailableMessage;
  if (!provider) return "This agent's provider is not registered.";
  if (provider.status === "not-installed") return `${provider.name} is not installed.`;
  return undefined;
}

/** Shown only while the agent's runtime status is `working`; the dots are its only ambient motion. */
function WorkingIndicator({
  agentName,
  currentAction
}: {
  agentName: string;
  currentAction?: string;
}) {
  return (
    <m.li
      variants={fadeUp}
      initial="initial"
      animate="animate"
      exit="exit"
      className="flex h-5 items-center gap-2 pl-1 text-xs text-foreground-subtle"
      role="status"
    >
      <AnimatePresence mode="wait" initial={false}>
        <m.span
          key={currentAction ?? "thinking"}
          className="flex min-w-0 items-center gap-2"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1, transition: transitions.enter }}
          exit={{ opacity: 0, transition: transitions.exit }}
        >
          {currentAction ? (
            <>
              <Terminal size={12} className="shrink-0" aria-hidden="true" />
              <span className="truncate font-mono">{currentAction}</span>
            </>
          ) : (
            <>
              <WorkingDots />
              <span>{agentName} is working</span>
            </>
          )}
        </m.span>
      </AnimatePresence>
    </m.li>
  );
}

function WorkingDots() {
  return (
    <span className="flex items-center gap-[3px]" aria-hidden="true" data-testid="working-dots">
      {[0, 1, 2].map((index) => (
        <m.span
          key={index}
          className="size-1 rounded-full bg-foreground-subtle"
          initial={{ opacity: 0.35, y: 0 }}
          animate={workingDotMotion.keyframes}
          transition={workingDotMotion.transition(index)}
        />
      ))}
    </span>
  );
}

function MessageEntry({
  message,
  agentName,
  fresh
}: {
  message: ConversationMessage;
  agentName: string;
  fresh: boolean;
}) {
  const time = formatConversationTime(message.createdAt);
  const motionProps = {
    variants: message.role === "user" ? userMessageIn : messageIn,
    initial: fresh ? "initial" : false,
    animate: "animate"
  } as const;
  if (message.role === "system") {
    return (
      <m.li {...motionProps} className="flex items-start gap-3 text-xs text-foreground-subtle">
        <Info size={13} className="mt-0.5 shrink-0" aria-hidden="true" />
        <span className="min-w-0 flex-1 whitespace-pre-wrap">{message.content}</span>
        <span className="shrink-0 text-xs-plus text-foreground-faint">{time}</span>
      </m.li>
    );
  }
  if (message.role === "user") {
    return (
      <m.li {...motionProps} className="flex justify-end">
        <div className="max-w-[80%] rounded-2xl rounded-br-md bg-muted px-3.5 py-2">
          <p className="whitespace-pre-wrap text-sm text-foreground">{message.content}</p>
          <p className="mt-1 text-right text-xs-plus text-foreground-faint">{time}</p>
        </div>
      </m.li>
    );
  }
  return (
    <m.li {...motionProps}>
      <p className="mb-1 text-xs-plus text-foreground-faint">
        <span className="font-medium text-foreground-muted">{agentName}</span> · {time}
      </p>
      <p className="whitespace-pre-wrap text-sm leading-6 text-foreground-secondary">
        {message.content}
      </p>
    </m.li>
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
    <li className="flex items-center gap-3 text-xs">
      <Icon size={13} className="shrink-0 text-foreground-faint" aria-hidden="true" />
      <span className="min-w-0 flex-1 text-foreground-muted">{text}</span>
      <span className="shrink-0 text-xs-plus text-foreground-faint">{time}</span>
    </li>
  );
}

function Composer({
  agentName,
  working,
  unavailableReason,
  error,
  onSend,
  onCancel
}: {
  agentName: string;
  working: boolean;
  unavailableReason?: string;
  error?: string;
  onSend: (content: string) => Promise<boolean>;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const disabled = Boolean(unavailableReason) || working || sending;

  async function submit() {
    if (disabled || !draft.trim()) return;
    setSending(true);
    if (await onSend(draft)) setDraft("");
    setSending(false);
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void submit();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      void submit();
    }
  }

  const statusText =
    error ??
    unavailableReason ??
    (working ? `${agentName} is working.` : "Enter to send · Shift+Enter for a new line");
  return (
    <div className="shrink-0 px-5 pb-3">
      <form className="mx-auto max-w-3xl" onSubmit={handleSubmit}>
        <div
          className={cn(
            "flex items-end gap-2 rounded-2xl border border-border bg-card py-2 pl-4 pr-2",
            unavailableReason && "opacity-80"
          )}
        >
          <textarea
            rows={1}
            value={draft}
            disabled={Boolean(unavailableReason)}
            aria-label={`Message ${agentName}`}
            aria-describedby="composer-status"
            placeholder={`Message ${agentName}`}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={handleKeyDown}
            className="max-h-40 min-h-8 min-w-0 flex-1 resize-none bg-transparent py-1.5 text-sm text-foreground outline-none placeholder:text-foreground-faint disabled:cursor-not-allowed"
          />
          <AnimatePresence mode="wait" initial={false}>
            {working ? (
              <m.span
                key="stop"
                variants={iconSwap}
                initial="initial"
                animate="animate"
                exit="exit"
              >
                <Button
                  type="button"
                  size="icon"
                  variant="secondary"
                  className="size-8 rounded-full"
                  aria-label="Stop"
                  title="Stop"
                  onClick={onCancel}
                >
                  <Square size={12} fill="currentColor" />
                </Button>
              </m.span>
            ) : (
              <m.span
                key="send"
                variants={iconSwap}
                initial="initial"
                animate="animate"
                exit="exit"
              >
                <Button
                  type="submit"
                  size="icon"
                  className="size-8 rounded-full"
                  aria-label="Send"
                  title="Send"
                  disabled={disabled || !draft.trim()}
                >
                  {sending ? <Loader2 size={14} className="animate-spin" /> : <ArrowUp size={15} />}
                </Button>
              </m.span>
            )}
          </AnimatePresence>
        </div>
        <p
          id="composer-status"
          role={error ? "alert" : undefined}
          className={cn(
            "mt-2 min-h-4 text-center text-xs-plus",
            error ? "text-danger-foreground" : "text-foreground-faint"
          )}
        >
          <AnimatePresence mode="wait" initial={false}>
            <m.span
              key={statusText}
              className="inline-block"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1, transition: transitions.enter }}
              exit={{ opacity: 0, transition: transitions.exit }}
            >
              {statusText}
            </m.span>
          </AnimatePresence>
        </p>
      </form>
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
