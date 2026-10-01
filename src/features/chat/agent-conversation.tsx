import { AnimatePresence, m, useReducedMotion } from "motion/react";
import {
  ArrowUp,
  Eraser,
  Info,
  Loader2,
  MessageSquare,
  Plus,
  RotateCcw,
  Sparkles,
  Square,
  Terminal,
  Trash2
} from "lucide-react";
import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { AgentDetails } from "@/features/agents/agent-details";
import { Button } from "@/components/ui/button";
import { Markdown } from "@/components/ui/markdown";
import { StatusBadge } from "@/components/ui/status-badge";
import { useConversation } from "@/hooks/use-conversation";
import { useFreshIds } from "@/hooks/use-fresh-ids";
import { isTauriRuntime, messagingUnavailableMessage } from "@/lib/desktop-api";
import {
  applyMention,
  mentionQuery,
  mentionSuggestions,
  type MentionCandidate
} from "@/lib/mentions";
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

/** A header action that asks for confirmation inline before it runs. */
type ConfirmedAction = "reset" | "clear" | "delete";

export function AgentConversation({
  agent,
  agents = [],
  provider,
  onAgentUpdated,
  onDelete
}: {
  agent: Agent;
  agents?: Agent[];
  provider?: ProviderSummary;
  onAgentUpdated?: (agent: Agent) => void;
  /** Deletes the agent; resolves to whether it was deleted. */
  onDelete?: (agent: Agent) => Promise<boolean>;
}) {
  const [showDetails, setShowDetails] = useState(false);
  const [confirming, setConfirming] = useState<ConfirmedAction>();
  const [busy, setBusy] = useState(false);
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

  const actions: Record<
    ConfirmedAction,
    {
      prompt: string;
      label: string;
      icon: typeof RotateCcw;
      variant: "secondary" | "danger";
      run: () => Promise<boolean>;
    }
  > = {
    reset: {
      prompt: "Clear the agent's context?",
      label: "New session",
      icon: RotateCcw,
      variant: "secondary",
      run: conversation.resetSession
    },
    clear: {
      prompt: "Delete every message in this conversation?",
      label: "Clear",
      icon: Eraser,
      variant: "danger",
      run: conversation.clear
    },
    delete: {
      prompt: `Delete ${agent.name} and its history?`,
      label: "Delete",
      icon: Trash2,
      variant: "danger",
      run: () => onDelete?.(agent) ?? Promise.resolve(false)
    }
  };

  async function runConfirmed(action: ConfirmedAction) {
    setBusy(true);
    const done = await actions[action].run();
    setBusy(false);
    if (done) setConfirming(undefined);
  }

  const headerButton =
    "grid size-8 place-items-center rounded-md text-foreground-subtle transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent";
  const blocked = working || Boolean(unavailableReason);

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
        <div className="ml-auto flex items-center gap-1">
          {confirming ? (
            <ConfirmPrompt
              prompt={actions[confirming].prompt}
              label={actions[confirming].label}
              icon={actions[confirming].icon}
              variant={actions[confirming].variant}
              busy={busy}
              disabled={working}
              onConfirm={() => void runConfirmed(confirming)}
              onCancel={() => setConfirming(undefined)}
            />
          ) : (
            <>
              <button
                type="button"
                onClick={() => setConfirming("reset")}
                disabled={blocked}
                aria-label="Start a new session"
                title={
                  working
                    ? "A new session can start once the current turn ends"
                    : "New session: the next message starts without earlier context"
                }
                className={headerButton}
              >
                <RotateCcw size={15} strokeWidth={1.8} />
              </button>
              <button
                type="button"
                onClick={() => setConfirming("clear")}
                disabled={blocked}
                aria-label="Clear conversation"
                title={
                  working
                    ? "The conversation can be cleared once the current turn ends"
                    : "Clear conversation: delete every message and start over"
                }
                className={headerButton}
              >
                <Eraser size={15} strokeWidth={1.8} />
              </button>
              {onDelete ? (
                <button
                  type="button"
                  onClick={() => setConfirming("delete")}
                  disabled={blocked}
                  aria-label="Delete agent"
                  title={
                    working ? "The agent can be deleted once the current turn ends" : "Delete agent"
                  }
                  className={headerButton}
                >
                  <Trash2 size={15} strokeWidth={1.8} />
                </button>
              ) : null}
            </>
          )}
          <button
            type="button"
            onClick={() => setShowDetails((current) => !current)}
            aria-label={showDetails ? "Hide agent details" : "Show agent details"}
            aria-pressed={showDetails}
            title="Agent details"
            className={cn(
              "grid size-8 place-items-center rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              showDetails
                ? "bg-accent text-foreground"
                : "text-foreground-subtle hover:bg-muted hover:text-foreground"
            )}
          >
            <Info size={16} strokeWidth={1.8} />
          </button>
        </div>
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
                sourceAgent={agents.find((candidate) => candidate.id === message.sourceAgentId)}
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
export function WorkingIndicator({
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
  sourceAgent,
  fresh
}: {
  message: ConversationMessage;
  agentName: string;
  sourceAgent?: Agent;
  fresh: boolean;
}) {
  const time = formatConversationTime(message.createdAt);
  const motionProps = {
    variants: message.role === "user" ? userMessageIn : messageIn,
    initial: fresh ? "initial" : false,
    animate: "animate"
  } as const;
  if (message.role === "system" && sourceAgent) {
    return (
      <m.li {...motionProps} className="flex items-start gap-3">
        <AgentAvatar
          color={sourceAgent.identityColor}
          variant={sourceAgent.avatarVariant}
          seed={sourceAgent.id}
          status={sourceAgent.status}
          size="sm"
        />
        <div className="min-w-0 max-w-[85%] rounded-xl rounded-tl-md border border-border/80 bg-card/60 px-3.5 py-2.5">
          <p className="mb-1.5 text-xs-plus text-foreground-faint">
            Message from{" "}
            <span className="font-medium text-foreground-muted">{sourceAgent.name}</span>
            <span aria-hidden="true"> · </span>
            {time}
          </p>
          <Markdown
            content={incomingAgentMessageBody(message.content)}
            className="text-sm leading-6 text-foreground-secondary"
          />
        </div>
      </m.li>
    );
  }
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
      <Markdown content={message.content} className="text-sm leading-6 text-foreground-secondary" />
    </m.li>
  );
}

/** The first paragraph is the delivery instruction sent to the provider, not message content. */
function incomingAgentMessageBody(content: string): string {
  const separator = content.indexOf("\n\n");
  return separator >= 0 ? content.slice(separator + 2) : content;
}

/** Inline confirmation for a header action, shown in place of the header buttons. */
export function ConfirmPrompt({
  prompt,
  label,
  icon: Icon,
  variant,
  busy,
  disabled = false,
  onConfirm,
  onCancel
}: {
  prompt: string;
  label: string;
  icon: typeof RotateCcw;
  variant: "secondary" | "danger";
  busy: boolean;
  disabled?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <>
      <span className="mr-1 text-xs text-foreground-subtle">{prompt}</span>
      <Button size="sm" variant={variant} disabled={busy || disabled} onClick={onConfirm}>
        {busy ? <Loader2 size={13} className="animate-spin" /> : <Icon size={13} />}
        {label}
      </Button>
      <Button size="sm" variant="ghost" onClick={onCancel}>
        Cancel
      </Button>
    </>
  );
}

export function TimelineEntry({
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

export function Composer({
  agentName,
  working,
  workingStatus,
  unavailableReason,
  error,
  mentionCandidates,
  onSend,
  onCancel
}: {
  /** Who the message goes to: an agent or a group. */
  agentName: string;
  working: boolean;
  /** Replaces the default "<name> is working." status while `working`. */
  workingStatus?: string;
  unavailableReason?: string;
  error?: string;
  /** Names suggested after typing `@`; without them there is no autocomplete. */
  mentionCandidates?: MentionCandidate[];
  onSend: (content: string) => Promise<boolean>;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [caret, setCaret] = useState(0);
  const [highlighted, setHighlighted] = useState(0);
  const [dismissedAt, setDismissedAt] = useState<number>();
  const textarea = useRef<HTMLTextAreaElement>(null);
  const disabled = Boolean(unavailableReason) || working || sending;
  const typed = mentionCandidates?.length ? mentionQuery(draft, caret) : undefined;
  const suggestions =
    typed && typed.start !== dismissedAt
      ? mentionSuggestions(typed.query, mentionCandidates ?? [])
      : [];
  const suggesting = suggestions.length > 0;
  const activeSuggestion = suggestions[Math.min(highlighted, suggestions.length - 1)];

  function updateDraft(value: string, nextCaret: number) {
    setDraft(value);
    setCaret(nextCaret);
    setHighlighted(0);
  }

  function choose(candidate: MentionCandidate) {
    if (!typed) return;
    const next = applyMention(draft, caret, typed.start, candidate);
    updateDraft(next.text, next.caret);
    requestAnimationFrame(() => {
      textarea.current?.focus();
      textarea.current?.setSelectionRange(next.caret, next.caret);
    });
  }

  async function submit() {
    if (disabled || !draft.trim()) return;
    setSending(true);
    if (await onSend(draft)) updateDraft("", 0);
    setSending(false);
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void submit();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (suggesting && !event.nativeEvent.isComposing) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const step = event.key === "ArrowDown" ? 1 : -1;
        setHighlighted((current) => (current + step + suggestions.length) % suggestions.length);
        return;
      }
      if ((event.key === "Enter" && !event.shiftKey) || event.key === "Tab") {
        event.preventDefault();
        choose(activeSuggestion);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        setDismissedAt(typed?.start);
        return;
      }
    }
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      void submit();
    }
  }

  const statusText =
    error ??
    unavailableReason ??
    (working
      ? (workingStatus ?? `${agentName} is working.`)
      : "Enter to send · Shift+Enter for a new line");
  return (
    <div className="shrink-0 px-5 pb-3">
      <form className="relative mx-auto max-w-3xl" onSubmit={handleSubmit}>
        {suggesting ? (
          <ul
            id="mention-suggestions"
            role="listbox"
            aria-label="Mention a member"
            className="absolute bottom-full left-0 z-10 mb-1 w-64 overflow-hidden rounded-lg border border-border bg-card p-1 shadow-panel"
          >
            {suggestions.map((candidate) => (
              <li
                key={candidate.id}
                id={`mention-${candidate.id}`}
                role="option"
                aria-selected={candidate === activeSuggestion}
                // Keep focus in the textarea while picking with the mouse.
                onMouseDown={(event) => {
                  event.preventDefault();
                  choose(candidate);
                }}
                className={cn(
                  "flex cursor-pointer items-baseline gap-2 rounded-md px-2 py-1.5 text-sm",
                  candidate === activeSuggestion
                    ? "bg-accent text-foreground"
                    : "text-foreground-secondary"
                )}
              >
                <span className="truncate font-medium">{candidate.name}</span>
                {candidate.detail ? (
                  <span className="truncate text-xs text-foreground-faint">{candidate.detail}</span>
                ) : null}
              </li>
            ))}
          </ul>
        ) : null}
        <div
          className={cn(
            "flex items-end gap-2 rounded-2xl border border-border bg-card py-2 pl-4 pr-2",
            unavailableReason && "opacity-80"
          )}
        >
          <textarea
            ref={textarea}
            rows={1}
            value={draft}
            disabled={Boolean(unavailableReason)}
            aria-label={`Message ${agentName}`}
            aria-describedby="composer-status"
            aria-autocomplete={mentionCandidates?.length ? "list" : undefined}
            aria-controls={suggesting ? "mention-suggestions" : undefined}
            aria-activedescendant={
              suggesting && activeSuggestion ? `mention-${activeSuggestion.id}` : undefined
            }
            placeholder={
              mentionCandidates?.length
                ? `Message ${agentName} · @ to mention`
                : `Message ${agentName}`
            }
            onChange={(event) =>
              updateDraft(
                event.target.value,
                event.target.selectionStart ?? event.target.value.length
              )
            }
            onSelect={(event) => setCaret(event.currentTarget.selectionStart ?? 0)}
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
