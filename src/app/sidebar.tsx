import { Activity, Bot, CheckSquare2, Command, ListTodo, Settings } from "lucide-react";
import { cn } from "@/lib/utils";

export type ViewId = "agents" | "tasks" | "activity" | "approvals" | "settings";

const navigation = [
  { id: "agents", label: "Agents", icon: Bot },
  { id: "tasks", label: "Tasks", icon: ListTodo },
  { id: "activity", label: "Activity", icon: Activity },
  { id: "approvals", label: "Approvals", icon: CheckSquare2 },
  { id: "settings", label: "Settings", icon: Settings }
] satisfies { id: ViewId; label: string; icon: typeof Bot }[];

export function Sidebar({
  active,
  onNavigate,
  onOpenPalette
}: {
  active: ViewId;
  onNavigate: (view: ViewId) => void;
  onOpenPalette: () => void;
}) {
  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-zinc-900 bg-[#0b0b0d] px-3 pb-3 pt-4">
      <div className="mb-7 flex items-center gap-2 px-2">
        <div className="grid size-7 place-items-center rounded-md border border-zinc-700 bg-zinc-100">
          <Bot size={16} className="text-zinc-950" />
        </div>
        <span className="text-sm font-semibold tracking-tight text-zinc-100">Open Bots</span>
        <span className="rounded bg-zinc-900 px-1.5 py-0.5 text-[9px] uppercase tracking-wide text-zinc-600">
          alpha
        </span>
      </div>
      <nav className="space-y-0.5">
        {navigation.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => onNavigate(id)}
            className={cn(
              "flex h-8 w-full items-center gap-2.5 rounded-md px-2.5 text-xs transition-colors",
              active === id
                ? "bg-zinc-800/80 text-zinc-100"
                : "text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300"
            )}
          >
            <Icon size={14} strokeWidth={1.8} />
            {label}
            {id === "approvals" && <span className="ml-auto size-1.5 rounded-full bg-amber-400" />}
          </button>
        ))}
      </nav>
      <div className="mt-auto">
        <button
          onClick={onOpenPalette}
          className="flex h-9 w-full items-center gap-2 rounded-md border border-zinc-900 bg-zinc-950 px-2.5 text-xs text-zinc-600 transition hover:border-zinc-800 hover:text-zinc-400"
        >
          <Command size={13} /> Commands{" "}
          <kbd className="ml-auto rounded border border-zinc-800 px-1.5 py-0.5 font-sans text-[9px]">
            ⌘K
          </kbd>
        </button>
        <div className="mt-3 flex items-center gap-2 px-2 text-[10px] text-zinc-700">
          <span className="size-1.5 rounded-full bg-emerald-500" /> Local runtime
        </div>
      </div>
    </aside>
  );
}
