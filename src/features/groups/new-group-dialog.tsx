import { Loader2, X } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { AgentAvatar } from "@/features/agents/agent-avatar";
import { groupsUnavailableMessage } from "@/lib/desktop-api";
import type { Agent, NewGroupInput } from "@/types/domain";

/** Mirrors `MAX_GROUP_MEMBERS` in `src-tauri/src/domain/groups.rs`. */
export const maxGroupMembers = 12;
const focusableSelector =
  'button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface NewGroupDialogProps {
  open: boolean;
  available: boolean;
  agents: Agent[];
  error?: string;
  onClose: () => void;
  onSubmit: (input: NewGroupInput) => Promise<boolean>;
}

export function NewGroupDialog({
  open,
  available,
  agents,
  error,
  onClose,
  onSubmit
}: NewGroupDialogProps) {
  const dialog = useRef<HTMLDivElement>(null);
  const nameInput = useRef<HTMLInputElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  const [name, setName] = useState("");
  const [topic, setTopic] = useState("");
  const [memberIds, setMemberIds] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    previousFocus.current = document.activeElement as HTMLElement | null;
    nameInput.current?.focus();
    return () => previousFocus.current?.focus();
  }, [open]);

  if (!open) return null;

  const tooMany = memberIds.length > maxGroupMembers;
  const canSubmit =
    available &&
    !saving &&
    name.trim().length > 0 &&
    topic.trim().length > 0 &&
    memberIds.length > 0 &&
    !tooMany;

  function reset() {
    setName("");
    setTopic("");
    setMemberIds([]);
  }

  function resetAndClose() {
    if (saving) return;
    reset();
    onClose();
  }

  function toggleMember(agentId: string, checked: boolean) {
    // Keep the order agents were picked in: it is the order they answer in.
    setMemberIds((current) =>
      checked ? [...current, agentId] : current.filter((id) => id !== agentId)
    );
  }

  function handleDialogKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      resetAndClose();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = Array.from(
      dialog.current?.querySelectorAll<HTMLElement>(focusableSelector) ?? []
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit) return;
    setSaving(true);
    const created = await onSubmit({ name: name.trim(), topic: topic.trim(), memberIds });
    setSaving(false);
    if (created) {
      reset();
      onClose();
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-overlay p-5 backdrop-blur-sm"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) resetAndClose();
      }}
    >
      <div
        ref={dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-group-title"
        aria-describedby="new-group-description"
        className="w-full max-w-lg overflow-hidden rounded-xl border border-border bg-card shadow-panel"
        onKeyDown={handleDialogKeyDown}
      >
        <div className="flex items-start justify-between border-b border-border-subtle px-5 py-4">
          <div>
            <h2 id="new-group-title" className="font-semibold text-foreground">
              Create group
            </h2>
            <p id="new-group-description" className="mt-0.5 text-xs text-foreground-subtle">
              Bring several agents together to work on one topic.
            </p>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="-mr-2 -mt-2"
            onClick={resetAndClose}
            disabled={saving}
            aria-label="Close"
          >
            <X size={16} />
          </Button>
        </div>

        <form className="space-y-4 p-5" onSubmit={(event) => void handleSubmit(event)}>
          <Field label="Name" htmlFor="group-name">
            <Input
              ref={nameInput}
              id="group-name"
              required
              maxLength={80}
              placeholder="Release 0.4"
              value={name}
              disabled={!available || saving}
              onChange={(event) => setName(event.target.value)}
            />
          </Field>
          <Field label="Topic" htmlFor="group-topic">
            <textarea
              id="group-topic"
              rows={3}
              maxLength={2000}
              className="field-textarea resize-none"
              placeholder="What should the group work on?"
              value={topic}
              disabled={!available || saving}
              onChange={(event) => setTopic(event.target.value)}
            />
          </Field>
          <fieldset>
            <legend className="mb-1.5 block text-xs font-medium text-foreground-muted">
              Members
            </legend>
            {agents.length === 0 ? (
              <p className="text-xs text-foreground-faint">Create an agent first.</p>
            ) : (
              <ul className="max-h-56 space-y-0.5 overflow-y-auto rounded-md border border-border p-1">
                {agents.map((agent) => {
                  const checked = memberIds.includes(agent.id);
                  const position = memberIds.indexOf(agent.id) + 1;
                  return (
                    <li key={agent.id}>
                      <label className="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-muted">
                        <Checkbox
                          checked={checked}
                          disabled={!available || saving}
                          aria-label={agent.name}
                          onCheckedChange={(next) => toggleMember(agent.id, next)}
                        />
                        <AgentAvatar
                          color={agent.identityColor}
                          variant={agent.avatarVariant}
                          seed={agent.id}
                          status={agent.status}
                          size="sm"
                        />
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-sm text-foreground">
                            {agent.name}
                          </span>
                          <span className="block truncate text-xs text-foreground-subtle">
                            {agent.role}
                          </span>
                        </span>
                        {checked ? (
                          <span
                            className="text-xs-plus text-foreground-faint"
                            title="Speaking order"
                          >
                            {position}
                          </span>
                        ) : null}
                      </label>
                    </li>
                  );
                })}
              </ul>
            )}
          </fieldset>
          <p className="text-xs leading-5 text-foreground-faint">
            {tooMany
              ? `Groups can have up to ${maxGroupMembers} members.`
              : "Members answer one at a time, in the order you pick them. Mention @Name to ask only some of them."}
          </p>

          {!available ? (
            <p
              role="status"
              className="rounded-md bg-muted px-3 py-2 text-xs text-foreground-muted"
            >
              {groupsUnavailableMessage}
            </p>
          ) : null}
          {error ? (
            <p role="alert" className="text-xs text-danger-foreground">
              {error}
            </p>
          ) : null}

          <div className="flex justify-end gap-2 border-t border-border-subtle pt-4">
            <Button type="button" variant="ghost" onClick={resetAndClose} disabled={saving}>
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit}>
              {saving ? <Loader2 size={14} className="animate-spin" /> : null}
              {saving ? "Creating…" : "Create group"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}

function Field({
  label,
  htmlFor,
  children
}: {
  label: string;
  htmlFor: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label className="mb-1.5 block text-xs font-medium text-foreground-muted" htmlFor={htmlFor}>
        {label}
      </label>
      {children}
    </div>
  );
}
