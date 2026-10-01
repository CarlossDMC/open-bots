import { AnimatePresence, m } from "motion/react";
import { Moon, Sun } from "lucide-react";
import { useTheme } from "@/hooks/use-theme";
import { iconSwap } from "@/lib/motion";
import { oppositeTheme } from "@/lib/theme";

export function ThemeToggle() {
  const { resolved, toggle } = useTheme();
  const label = `Switch to ${oppositeTheme(resolved)} theme`;
  const Icon = resolved === "dark" ? Moon : Sun;
  return (
    <button
      type="button"
      onClick={toggle}
      aria-label={label}
      title={label}
      className="relative grid size-6 place-items-center overflow-hidden rounded-md text-foreground-subtle transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    >
      <AnimatePresence initial={false} mode="popLayout">
        <m.span
          key={resolved}
          variants={iconSwap}
          initial="initial"
          animate="animate"
          exit="exit"
          className="grid place-items-center"
        >
          <Icon size={13} strokeWidth={1.8} aria-hidden="true" />
        </m.span>
      </AnimatePresence>
    </button>
  );
}
