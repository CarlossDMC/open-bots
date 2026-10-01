import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...props }, ref) {
    return (
      <input
        ref={ref}
        className={cn(
          "h-9 w-full rounded-md border border-border bg-card px-3 text-sm text-foreground outline-none placeholder:text-foreground-faint focus:border-border-strong focus:ring-1 focus:ring-ring/40",
          className
        )}
        {...props}
      />
    );
  }
);
