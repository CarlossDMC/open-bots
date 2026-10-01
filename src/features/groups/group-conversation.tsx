import { AnimatePresence, m, useReducedMotion } from "motion/react";
import { Info, Loader2, Sparkles, Trash2, Users } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Markdown, MentionLink } from "@/components/ui/markdown";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { Composer, TimelineEntry, WorkingIndicator } from "@/features/chat/agent-conversation";
import { useFreshIds } from "@/hooks/use-fresh-ids";
import { useGroupConversation } from "@/hooks/use-group-conversation";
import { groupsUnavailableMessage, isTauriRuntime } from "@/lib/desktop-api";
import { splitMentions, type MentionCandidate } from "@/lib/mentions";
import { messageIn, userMessageIn } from "@/lib/motion";
import { formatConversationTime } from "@/lib/utils";
import type { Agent, Group, GroupMessage } from "@/types/domain";

const visibleMemberAvatars = 5;

export function GroupConversation({
  group,
  agents,
  onDelete,
  onOpenAgent
}: {
  group: Group;
  agents: Agent[];
  onDelete: (group: Group) => Promise<boolean>;
  /** Opens a mentioned member's own conversation. */
  onOpenAgent?: (agent: Agent) => void;
}) {
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const conversation = useGroupConversation(group.id);
  const members = group.memberIds
    .map((id) => agents.find((agent) => agent.id === id))
    .filter((agent): agent is Agent => Boolean(agent));
  const mentionCandidates = useMemo<MentionCandidate[]>(
    () =>
      group.memberIds.flatMap((id) => {
        const agent = agents.find((candidate) => candidate.id === id);
        return agent ? [{ id: agent.id, name: agent.name, detail: agent.role }] : [];
      }),
    [group.memberIds, agents]
  );
  function openMention(id: string) {
    const agent = agents.find((candidate) => candidate.id === id);
    if (agent) onOpenAgent?.(agent);
  }
  const speaker = agents.find((agent) => agent.id === group.round.queue[0]?.agentId);
  const active = group.round.queue.length > 0;
  const unavailableReason = isTauriRuntime() ? undefined : groupsUnavailableMessage;
  const endRef = useRef<HTMLLIElement>(null);
  const reduceMotion = useReducedMotion();
  const freshIds = useFreshIds(
    conversation.messages.map((message) => message.id),
    !conversation.loading
  );
  const hasFreshContent = freshIds.size > 0 || active;

  useEffect(() => {
    const behavior = hasFreshContent && !reduceMotion ? "smooth" : "auto";
    endRef.current?.scrollIntoView?.({ block: "end", behavior });
  }, [
    conversation.messages.length,
    conversation.currentAction,
    active,
    hasFreshContent,
    reduceMotion
  ]);

  async function confirmDelete() {
    setDeleting(true);
    const deleted = await onDelete(group);
    setDeleting(false);
    if (!deleted) setConfirmingDelete(false);
  }

  const createdBy = group.createdBy;
  const creator =
    createdBy.kind === "agent"
      ? agents.find((agent) => agent.id === createdBy.agentId)?.name
      : undefined;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="flex h-14 shrink-0 items-center gap-3 border-b border-border-subtle px-5">
        <span className="grid size-9 shrink-0 place-items-center rounded-lg border border-border bg-card text-foreground-muted">
          <Users size={15} strokeWidth={1.8} aria-hidden="true" />
        </span>
        <div className="min-w-0">
          <h1 className="truncate text-sm font-medium leading-5 text-foreground">{group.name}</h1>
          <p className="truncate text-xs-plus leading-4 text-foreground-faint" title={group.topic}>
            {group.topic}
          </p>
        </div>
        <div className="ml-auto flex items-center gap-2">
          <ul className="flex items-center -space-x-1.5" aria-label="Members">
            {members.slice(0, visibleMemberAvatars).map((member) => (
              <li key={member.id} title={`${member.name} · ${member.role}`}>
                <AgentAvatar
                  color={member.identityColor}
                  variant={member.avatarVariant}
                  seed={member.id}
                  status={member.status}
                  size="sm"
                />
              </li>
            ))}
            {members.length > visibleMemberAvatars ? (
              <li className="pl-2.5 text-xs-plus text-foreground-faint">
                +{members.length - visibleMemberAvatars}
              </li>
            ) : null}
          </ul>
          {confirmingDelete ? (
            <>
              <span className="text-xs text-foreground-subtle">Delete this group?</span>
              <Button
                size="sm"
                variant="danger"
                disabled={deleting}
                onClick={() => void confirmDelete()}
              >
                {deleting ? <Loader2 size={13} className="animate-spin" /> : <Trash2 size={13} />}
                Delete
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setConfirmingDelete(false)}>
                Cancel
              </Button>
            </>
          ) : (
            <button
              type="button"
              onClick={() => setConfirmingDelete(true)}
              disabled={Boolean(unavailableReason)}
              aria-label="Delete group"
              title="Delete group"
              className="grid size-8 place-items-center rounded-md text-foreground-subtle transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent"
            >
              <Trash2 size={15} strokeWidth={1.8} />
            </button>
          )}
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <ol className="mx-auto max-w-3xl space-y-5 px-8 py-8" aria-label="Group conversation">
          <TimelineEntry
            icon={Sparkles}
            text={`Created by ${creator ?? "you"} · ${members.length} member${members.length === 1 ? "" : "s"}`}
            time={formatConversationTime(group.createdAt)}
          />
          {conversation.messages.map((message) => (
            <GroupMessageEntry
              key={message.id}
              message={message}
              agents={agents}
              mentions={mentionCandidates}
              onMention={openMention}
              fresh={freshIds.has(message.id)}
            />
          ))}
          <AnimatePresence initial={false}>
            {active && speaker ? (
              <WorkingIndicator
                key={`working-${speaker.id}`}
                agentName={speaker.name}
                currentAction={conversation.currentAction}
              />
            ) : null}
          </AnimatePresence>
          <li ref={endRef} aria-hidden="true" />
        </ol>
      </div>
      <Composer
        agentName={group.name}
        working={active}
        workingStatus={speaker ? `${speaker.name} is answering.` : "Members are answering."}
        unavailableReason={unavailableReason}
        error={conversation.error}
        mentionCandidates={mentionCandidates}
        onSend={conversation.send}
        onCancel={() => void conversation.stop()}
      />
    </div>
  );
}

function GroupMessageEntry({
  message,
  agents,
  mentions,
  onMention,
  fresh
}: {
  message: GroupMessage;
  agents: Agent[];
  mentions: MentionCandidate[];
  onMention: (id: string) => void;
  fresh: boolean;
}) {
  const time = formatConversationTime(message.createdAt);
  const author = message.author;
  const motionProps = {
    variants: author.kind === "user" ? userMessageIn : messageIn,
    initial: fresh ? "initial" : false,
    animate: "animate"
  } as const;
  if (author.kind === "system") {
    return (
      <m.li {...motionProps} className="flex items-start gap-3 text-xs text-foreground-subtle">
        <Info size={13} className="mt-0.5 shrink-0" aria-hidden="true" />
        <span className="min-w-0 flex-1 whitespace-pre-wrap">{message.content}</span>
        <span className="shrink-0 text-xs-plus text-foreground-faint">{time}</span>
      </m.li>
    );
  }
  if (author.kind === "user") {
    return (
      <m.li {...motionProps} className="flex justify-end">
        <div className="max-w-[80%] rounded-2xl rounded-br-md bg-muted px-3.5 py-2">
          <p className="whitespace-pre-wrap text-sm text-foreground">
            {splitMentions(message.content, mentions).map((part, index) =>
              typeof part === "string" ? (
                part
              ) : (
                <MentionLink
                  key={index}
                  label={message.content.slice(part.start, part.end)}
                  onClick={() => onMention(part.candidate.id)}
                >
                  {message.content.slice(part.start, part.end)}
                </MentionLink>
              )
            )}
          </p>
          <p className="mt-1 text-right text-xs-plus text-foreground-faint">{time}</p>
        </div>
      </m.li>
    );
  }
  const agent = agents.find((candidate) => candidate.id === author.agentId);
  return (
    <m.li {...motionProps} className="flex items-start gap-3">
      {agent ? (
        <AgentAvatar
          color={agent.identityColor}
          variant={agent.avatarVariant}
          seed={agent.id}
          status={agent.status}
          size="sm"
        />
      ) : (
        <span className="size-9 shrink-0 rounded-lg bg-muted" aria-hidden="true" />
      )}
      <div className="min-w-0 flex-1">
        <p className="mb-1 text-xs-plus text-foreground-faint">
          <span className="font-medium text-foreground-muted">
            {agent?.name ?? "A former member"}
          </span>{" "}
          · {time}
        </p>
        <Markdown
          content={message.content}
          className="text-sm leading-6 text-foreground-secondary"
          mentions={mentions}
          onMention={onMention}
        />
      </div>
    </m.li>
  );
}
