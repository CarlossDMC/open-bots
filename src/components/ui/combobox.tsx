import { Check, ChevronsUpDown, Search } from "lucide-react";
import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent
} from "react";
import { createPortal } from "react-dom";
import { filterOptions } from "@/lib/combobox";
import { cn } from "@/lib/utils";

export interface ComboboxOption {
  value: string;
  label: string;
  /** Secondary line shown under the label; also matched by the search. */
  description?: string;
  disabled?: boolean;
}

interface ComboboxProps {
  value: string;
  options: ComboboxOption[];
  onChange: (value: string) => void;
  id?: string;
  "aria-label"?: string;
  placeholder?: string;
  searchPlaceholder?: string;
  emptyMessage?: string;
  disabled?: boolean;
  className?: string;
}

interface PopoverPosition {
  left: number;
  width: number;
  top?: number;
  bottom?: number;
}

/** Keeps the list clear of the viewport edge. */
const viewportMargin = 8;
/** Preferred list height before it flips above the trigger. */
const preferredListHeight = 280;

/** A searchable single-select following the WAI-ARIA combobox pattern. */
export function Combobox({
  value,
  options,
  onChange,
  id,
  "aria-label": ariaLabel,
  placeholder = "Select…",
  searchPlaceholder = "Search…",
  emptyMessage = "No matches.",
  disabled = false,
  className
}: ComboboxProps) {
  const generatedId = useId();
  const triggerId = id ?? `${generatedId}-trigger`;
  const listboxId = `${generatedId}-listbox`;
  const optionId = (index: number) => `${generatedId}-option-${index}`;
  const trigger = useRef<HTMLButtonElement>(null);
  const popover = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(-1);
  const [position, setPosition] = useState<PopoverPosition>();

  const selected = options.find((option) => option.value === value);
  const visible = useMemo(() => filterOptions(options, query), [options, query]);

  const close = useCallback((restoreFocus: boolean) => {
    setOpen(false);
    setQuery("");
    if (restoreFocus) trigger.current?.focus();
  }, []);

  function openList() {
    if (disabled) return;
    const selectedIndex = options.findIndex((option) => option.value === value);
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : firstEnabled(options));
    setOpen(true);
  }

  function choose(option: ComboboxOption | undefined) {
    if (!option || option.disabled) return;
    if (option.value !== value) onChange(option.value);
    close(true);
  }

  const place = useCallback(() => {
    const rect = trigger.current?.getBoundingClientRect();
    if (!rect) return;
    const below = window.innerHeight - rect.bottom - viewportMargin;
    const above = rect.top - viewportMargin;
    const flip = below < preferredListHeight && above > below;
    setPosition({
      left: rect.left,
      width: rect.width,
      ...(flip ? { bottom: window.innerHeight - rect.top + 4 } : { top: rect.bottom + 4 })
    });
  }, []);

  useLayoutEffect(() => {
    if (open) place();
    else setPosition(undefined);
  }, [open, place]);

  // The popover mounts once it has a position, so focus waits for that render.
  const placed = position !== undefined;
  useLayoutEffect(() => {
    if (open && placed) search.current?.focus();
  }, [open, placed]);

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!popover.current?.contains(target) && !trigger.current?.contains(target)) close(false);
    };
    window.addEventListener("pointerdown", onPointer);
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      window.removeEventListener("pointerdown", onPointer);
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [open, close, place]);

  useEffect(() => {
    if (!open || activeIndex < 0) return;
    list.current
      ?.querySelector(`[data-index="${activeIndex}"]`)
      ?.scrollIntoView?.({ block: "nearest" });
  }, [open, activeIndex]);

  function onTriggerKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
      event.preventDefault();
      openList();
    }
  }

  function onSearchKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        setActiveIndex((current) => step(visible, current, event.key === "ArrowDown" ? 1 : -1));
        break;
      }
      case "Home":
        event.preventDefault();
        setActiveIndex(firstEnabled(visible));
        break;
      case "End":
        event.preventDefault();
        setActiveIndex(step(visible, visible.length, -1));
        break;
      case "Enter":
        event.preventDefault();
        choose(visible[activeIndex]);
        break;
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        close(true);
        break;
      case "Tab":
        close(false);
        break;
    }
  }

  return (
    <>
      <button
        ref={trigger}
        id={triggerId}
        type="button"
        role="combobox"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listboxId : undefined}
        disabled={disabled}
        onClick={() => (open ? close(false) : openList())}
        onKeyDown={onTriggerKeyDown}
        className={cn(
          "flex h-9 w-full items-center gap-2 rounded-md border border-border bg-card px-3 text-left text-sm text-foreground outline-none transition-colors hover:border-border-strong focus-visible:border-border-strong focus-visible:ring-1 focus-visible:ring-ring/40 disabled:cursor-not-allowed disabled:opacity-50",
          open && "border-border-strong ring-1 ring-ring/40",
          className
        )}
      >
        <span className={cn("min-w-0 flex-1 truncate", !selected && "text-foreground-faint")}>
          {selected?.label ?? placeholder}
        </span>
        <ChevronsUpDown size={14} className="shrink-0 text-foreground-faint" aria-hidden="true" />
      </button>
      {open &&
        position &&
        createPortal(
          <div
            ref={popover}
            className="fixed z-[60] flex flex-col overflow-hidden rounded-lg border border-border bg-card shadow-panel animate-fade-in"
            style={{
              left: position.left,
              width: Math.max(position.width, 200),
              top: position.top,
              bottom: position.bottom,
              maxHeight: preferredListHeight
            }}
          >
            <div className="flex items-center gap-2 border-b border-border-subtle px-2.5">
              <Search size={13} className="shrink-0 text-foreground-faint" aria-hidden="true" />
              <input
                ref={search}
                role="searchbox"
                aria-label={searchPlaceholder}
                aria-controls={listboxId}
                aria-activedescendant={activeIndex >= 0 ? optionId(activeIndex) : undefined}
                autoComplete="off"
                spellCheck={false}
                value={query}
                placeholder={searchPlaceholder}
                onChange={(event) => {
                  const next = filterOptions(options, event.target.value);
                  setQuery(event.target.value);
                  setActiveIndex(firstEnabled(next));
                }}
                onKeyDown={onSearchKeyDown}
                className="h-8 min-w-0 flex-1 bg-transparent text-xs text-foreground outline-none placeholder:text-foreground-faint"
              />
            </div>
            <ul
              ref={list}
              id={listboxId}
              role="listbox"
              aria-labelledby={triggerId}
              className="min-h-0 flex-1 overflow-y-auto p-1"
            >
              {visible.length === 0 ? (
                <li className="px-2 py-2 text-xs text-foreground-faint" role="presentation">
                  {emptyMessage}
                </li>
              ) : (
                visible.map((option, index) => {
                  const isSelected = option.value === value;
                  return (
                    <li
                      key={option.value}
                      id={optionId(index)}
                      data-index={index}
                      role="option"
                      aria-selected={isSelected}
                      aria-disabled={option.disabled || undefined}
                      onPointerMove={() => !option.disabled && setActiveIndex(index)}
                      onPointerDown={(event) => event.preventDefault()}
                      onClick={() => choose(option)}
                      className={cn(
                        "flex cursor-default items-start gap-2 rounded-md px-2 py-1.5 text-xs",
                        index === activeIndex && "bg-muted",
                        option.disabled
                          ? "cursor-not-allowed text-foreground-faint"
                          : "text-foreground-secondary"
                      )}
                    >
                      <Check
                        size={13}
                        className={cn("mt-px shrink-0 text-foreground", !isSelected && "invisible")}
                        aria-hidden="true"
                      />
                      <span className="min-w-0">
                        <span className="block truncate text-foreground">{option.label}</span>
                        {option.description && (
                          <span className="mt-0.5 block text-2xs leading-4 text-foreground-faint">
                            {option.description}
                          </span>
                        )}
                      </span>
                    </li>
                  );
                })
              )}
            </ul>
          </div>,
          document.body
        )}
    </>
  );
}

function firstEnabled(options: ComboboxOption[]): number {
  return options.findIndex((option) => !option.disabled);
}

/** Moves to the next enabled option in `direction`, staying put at either end. */
function step(options: ComboboxOption[], from: number, direction: 1 | -1): number {
  for (let index = from + direction; index >= 0 && index < options.length; index += direction) {
    if (!options[index].disabled) return index;
  }
  return from;
}
