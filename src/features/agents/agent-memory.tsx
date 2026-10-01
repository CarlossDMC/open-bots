import { Brain, Loader2, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAgentMemories } from "@/hooks/use-agent-memories";
import { formatRelativeTime } from "@/lib/utils";

const maxMemoryLength = 2000;

export function AgentMemorySection({ agentId }: { agentId: string }) {
  const { memories, loading, error, add, remove } = useAgentMemories(agentId);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!draft.trim() || saving) return;
    setSaving(true);
    if (await add(draft)) setDraft("");
    setSaving(false);
  }

  return (
    <section className="panel lg:col-span-5" aria-labelledby="agent-memory-title">
      <h2 id="agent-memory-title" className="section-title">
        <Brain size={14} /> Memory
      </h2>
      <p className="mb-3 text-xs text-foreground-faint">
        Durable notes stored locally. They are shared with the provider when a new session starts.
      </p>
      <form className="flex gap-2" onSubmit={(event) => void handleSubmit(event)}>
        <Input
          aria-label="New memory"
          placeholder="Remember that…"
          value={draft}
          maxLength={maxMemoryLength}
          onChange={(event) => setDraft(event.target.value)}
        />
        <Button type="submit" size="sm" className="h-9" disabled={!draft.trim() || saving}>
          {saving ? <Loader2 size={13} className="animate-spin" /> : null}
          Add
        </Button>
      </form>
      {error ? (
        <p role="alert" className="mt-2 text-xs text-danger-foreground">
          {error}
        </p>
      ) : null}
      {loading ? (
        <p className="mt-4 text-xs text-foreground-faint">Loading memories…</p>
      ) : memories.length === 0 ? (
        <p className="mt-4 text-xs text-foreground-faint">No memories yet.</p>
      ) : (
        <ul className="mt-3 divide-y divide-border-subtle">
          {memories.map((memory) => (
            <li key={memory.id} className="group flex items-start gap-3 py-2">
              <p className="min-w-0 flex-1 whitespace-pre-wrap text-sm text-foreground-secondary">
                {memory.content}
              </p>
              <time className="shrink-0 pt-0.5 text-2xs tabular-nums text-foreground-faint">
                {formatRelativeTime(memory.createdAt)}
              </time>
              <Button
                variant="ghost"
                size="icon"
                className="size-6 shrink-0"
                aria-label="Remove memory"
                onClick={() => void remove(memory)}
              >
                <Trash2 size={12} />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
