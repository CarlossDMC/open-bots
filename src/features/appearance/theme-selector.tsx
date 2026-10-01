import { Monitor, Moon, Sun } from "lucide-react";
import { useRef, type KeyboardEvent } from "react";
import { useTheme } from "@/hooks/use-theme";
import type { ThemePreference } from "@/lib/theme";
import { cn } from "@/lib/utils";

const options = [
  { value: "system", label: "System", icon: Monitor },
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon }
] satisfies { value: ThemePreference; label: string; icon: typeof Sun }[];

const keyOffsets: Record<string, number> = {
  ArrowRight: 1,
  ArrowDown: 1,
  ArrowLeft: -1,
  ArrowUp: -1
};

export function ThemeSelector() {
  const { preference, setPreference } = useTheme();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const offset = keyOffsets[event.key];
    if (!offset) return;
    event.preventDefault();
    const next = (index + offset + options.length) % options.length;
    setPreference(options[next].value);
    buttons.current[next]?.focus();
  }

  return (
    <div
      role="radiogroup"
      aria-label="Theme"
      className="inline-flex rounded-md border border-border bg-muted p-0.5"
    >
      {options.map(({ value, label, icon: Icon }, index) => {
        const checked = preference === value;
        return (
          <button
            key={value}
            ref={(element) => {
              buttons.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={checked}
            tabIndex={checked ? 0 : -1}
            onClick={() => setPreference(value)}
            onKeyDown={(event) => handleKeyDown(event, index)}
            className={cn(
              "flex h-6 items-center gap-1.5 rounded px-2 text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              checked ? "bg-card text-foreground" : "text-foreground-subtle hover:text-foreground"
            )}
          >
            <Icon size={12} aria-hidden="true" />
            {label}
          </button>
        );
      })}
    </div>
  );
}
