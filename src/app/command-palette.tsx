import { Bot, ListTodo, Search, X } from "lucide-react";
import { useEffect } from "react";
import type { ViewId } from "./sidebar";

export function CommandPalette({
  open,
  onClose,
  onNavigate,
  onCreateAgent
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (view: ViewId) => void;
  onCreateAgent: () => void;
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
      className="fixed inset-0 z-[60] flex justify-center bg-black/60 pt-[16vh] backdrop-blur-sm"
      onMouseDown={onClose}
    >
      <div
        className="h-fit w-full max-w-lg overflow-hidden rounded-xl border border-zinc-800 bg-zinc-950 shadow-panel"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="flex items-center gap-3 border-b border-zinc-900 px-4">
          <Search size={16} className="text-zinc-600" />
          <input
            className="h-12 flex-1 bg-transparent text-sm text-zinc-200 outline-none placeholder:text-zinc-600"
            placeholder="Type a command…"
            autoFocus
          />
          <X size={14} className="cursor-pointer text-zinc-700" onClick={onClose} />
        </div>
        <div className="p-2">
          <p className="px-2 py-1.5 text-[10px] font-medium uppercase tracking-widest text-zinc-700">
            Actions
          </p>
          <Command
            icon={Bot}
            label="Create Agent"
            shortcut="A"
            onClick={() => execute(onCreateAgent)}
          />
          <Command
            icon={ListTodo}
            label="View Tasks"
            shortcut="T"
            onClick={() => execute(() => onNavigate("tasks"))}
          />
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
  shortcut: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-sm text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100"
    >
      <Icon size={15} />
      {label}
      <kbd className="ml-auto text-[10px] text-zinc-700">{shortcut}</kbd>
    </button>
  );
}
