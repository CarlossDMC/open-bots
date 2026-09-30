import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import type { ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-zinc-100 text-zinc-950 hover:bg-white",
        secondary: "border border-zinc-800 bg-zinc-900 text-zinc-200 hover:bg-zinc-800",
        ghost: "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100",
        danger: "border border-red-900/70 bg-red-950/40 text-red-300 hover:bg-red-950/70"
      },
      size: { default: "h-9 px-3.5", sm: "h-8 px-3 text-xs", icon: "size-9" }
    },
    defaultVariants: { variant: "default", size: "default" }
  }
);

interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export function Button({ className, variant, size, asChild = false, ...props }: ButtonProps) {
  const Component = asChild ? Slot : "button";
  return <Component className={cn(buttonVariants({ variant, size, className }))} {...props} />;
}
