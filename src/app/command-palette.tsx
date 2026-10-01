import {
  Activity,
  Bot,
  CheckSquare2,
  ListTodo,
  MessageSquare,
  Search,
  Settings,
  SunMoon,
  X
} from "lucide-react";
import { useEffect } from "react";
import type { ViewId } from "./sidebar";

export function CommandPalette({
  open,
  onClose,
  onNavigate,
  onCreateAgent,
  onToggleTheme
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (view: ViewId) => void;
  onCreateAgent: () => void;
  onToggleTheme: () => void;
}) {
  useEffect(() => {
    if (!open) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);
  if (!open) return null;
  const execute = (action: () => void) => {
    action();
    onClose();
  };
  return (
    <div
      className="fixed inset-0 z-[60] flex justify-center bg-overlay pt-[16vh] backdrop-blur-sm"
      onMouseDown={onClose}
    >
      <div
        className="h-fit w-full max-w-lg overflow-hidden rounded-xl border border-border bg-card shadow-panel"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="flex items-center gap-3 border-b border-border-subtle px-4">
          <Search size={16} className="text-foreground-faint" />
          <input
            className="h-12 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-foreground-faint"
            placeholder="Type a command…"
            autoFocus
          />
          <X size={14} className="cursor-pointer text-foreground-faint" onClick={onClose} />
        </div>
        <div className="p-2">
          <p className="px-2 py-1.5 text-2xs font-medium uppercase tracking-widest text-foreground-faint">
            Actions
          </p>
          <Command
            icon={Bot}
            label="Create Agent"
            shortcut="A"
            onClick={() => execute(onCreateAgent)}
          />
          <Command
            icon={MessageSquare}
            label="View Agents"
            onClick={() => execute(() => onNavigate("chat"))}
          />
          <Command
            icon={ListTodo}
            label="View Tasks"
            shortcut="T"
            onClick={() => execute(() => onNavigate("tasks"))}
          />
          <Command
            icon={Activity}
            label="View Activity"
            onClick={() => execute(() => onNavigate("activity"))}
          />
          <Command
            icon={CheckSquare2}
            label="View Approvals"
            onClick={() => execute(() => onNavigate("approvals"))}
          />
          <Command
            icon={Settings}
            label="Open Settings"
            onClick={() => execute(() => onNavigate("settings"))}
          />
          <Command icon={SunMoon} label="Toggle Theme" onClick={() => execute(onToggleTheme)} />
        </div>
      </div>
    </div>
  );
}

function Command({
  icon: Icon,
  label,
  shortcut,
  onClick
}: {
  icon: typeof Bot;
  label: string;
  shortcut?: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-sm text-foreground-muted hover:bg-muted hover:text-foreground"
    >
      <Icon size={15} />
      {label}
      {shortcut && <kbd className="ml-auto text-2xs text-foreground-faint">{shortcut}</kbd>}
    </button>
  );
}
