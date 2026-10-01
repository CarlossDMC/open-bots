import { Check } from "lucide-react";
import type { KeyboardEvent } from "react";
import { cn } from "@/lib/utils";

interface CheckboxProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  id?: string;
  "aria-label"?: string;
  disabled?: boolean;
  className?: string;
}

/** A styled checkbox. It works inside a wrapping `<label>` or with `aria-label`. */
export function Checkbox({
  checked,
  onCheckedChange,
  id,
  "aria-label": ariaLabel,
  disabled = false,
  className
}: CheckboxProps) {
  // Checkboxes toggle with Space only; Enter would otherwise click the underlying button.
  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "Enter") event.preventDefault();
  }

  return (
    <button
      id={id}
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={() => onCheckedChange(!checked)}
      onKeyDown={onKeyDown}
      className={cn(
        "grid size-4 shrink-0 place-items-center rounded-[4px] border transition-colors duration-fast focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50",
        checked
          ? "border-foreground bg-foreground text-background"
          : "border-border-strong bg-card hover:border-foreground-subtle",
        className
      )}
    >
      <Check
        size={11}
        strokeWidth={3}
        aria-hidden="true"
        className={cn(
          "transition-[opacity,transform] duration-fast motion-reduce:transition-none",
          checked ? "scale-100 opacity-100" : "scale-50 opacity-0"
        )}
      />
    </button>
  );
}
