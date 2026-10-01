import { Loader2, X } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { Button } from "@/components/ui/button";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { Input } from "@/components/ui/input";
import { tasksUnavailableMessage } from "@/lib/desktop-api";
import type { Agent, NewTaskInput } from "@/types/domain";

const unassigned = "";
const focusableSelector =
  'button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface NewTaskDialogProps {
  open: boolean;
  available: boolean;
  agents: Agent[];
  error?: string;
  onClose: () => void;
  onSubmit: (input: NewTaskInput) => Promise<boolean>;
}

export function NewTaskDialog({
  open,
  available,
  agents,
  error,
  onClose,
  onSubmit
}: NewTaskDialogProps) {
  const dialog = useRef<HTMLDivElement>(null);
  const titleInput = useRef<HTMLInputElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [assignee, setAssignee] = useState(unassigned);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    previousFocus.current = document.activeElement as HTMLElement | null;
    titleInput.current?.focus();
    return () => previousFocus.current?.focus();
  }, [open]);

  if (!open) return null;

  const options: ComboboxOption[] = [
    { value: unassigned, label: "Unassigned", description: "Keep the task pending" },
    ...agents.map((agent) => ({
      value: agent.id,
      label: agent.name,
      description: agent.role
    }))
  ];
  const canSubmit = available && !saving && title.trim().length > 0;

  function resetAndClose() {
    if (saving) return;
    setTitle("");
    setDescription("");
    setAssignee(unassigned);
    onClose();
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
    const created = await onSubmit({
      title: title.trim(),
      description: description.trim(),
      assignedAgentId: assignee || undefined
    });
    setSaving(false);
    if (created) {
      setTitle("");
      setDescription("");
      setAssignee(unassigned);
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
        aria-labelledby="new-task-title"
        aria-describedby="new-task-description"
        className="w-full max-w-lg overflow-hidden rounded-xl border border-border bg-card shadow-panel"
        onKeyDown={handleDialogKeyDown}
      >
        <div className="flex items-start justify-between border-b border-border-subtle px-5 py-4">
          <div>
            <h2 id="new-task-title" className="font-semibold text-foreground">
              Create task
            </h2>
            <p id="new-task-description" className="mt-0.5 text-xs text-foreground-subtle">
              Add a persistent unit of work for you or an agent.
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
          <Field label="Title" htmlFor="task-title">
            <Input
              ref={titleInput}
              id="task-title"
              required
              maxLength={200}
              placeholder="What needs to be done?"
              value={title}
              disabled={!available || saving}
              onChange={(event) => setTitle(event.target.value)}
            />
          </Field>
          <Field label="Description" htmlFor="task-description-input">
            <textarea
              id="task-description-input"
              rows={4}
              maxLength={4000}
              className="field-textarea resize-none"
              placeholder="Add context, requirements, or an expected outcome."
              value={description}
              disabled={!available || saving}
              onChange={(event) => setDescription(event.target.value)}
            />
          </Field>
          <Field label="Assigned agent" htmlFor="task-assignee">
            <Combobox
              id="task-assignee"
              aria-label="Assigned agent"
              value={assignee}
              disabled={!available || saving}
              searchPlaceholder="Search agents…"
              options={options}
              onChange={setAssignee}
            />
          </Field>
          <p className="text-xs leading-5 text-foreground-faint">
            {assignee
              ? "Assigned tasks enter the selected agent's queue."
              : "Unassigned tasks remain pending until an agent takes ownership."}
          </p>

          {!available ? (
            <p
              role="status"
              className="rounded-md bg-muted px-3 py-2 text-xs text-foreground-muted"
            >
              {tasksUnavailableMessage}
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
              {saving ? "Creating…" : "Create task"}
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
