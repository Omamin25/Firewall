import { cva, type VariantProps } from "class-variance-authority";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

const button = cva(
  "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium transition-opacity duration-fast ease-enter focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50",
  {
    variants: {
      variant: {
        primary: "bg-secondary text-secondary-foreground hover:opacity-90 active:opacity-80",
        accent: "bg-primary text-primary-foreground hover:opacity-90 active:opacity-80",
        outline: "border border-border-strong bg-transparent text-foreground hover:bg-surface-muted",
        ghost: "bg-transparent text-foreground hover:bg-surface-muted",
        destructive: "bg-danger text-danger-foreground hover:opacity-90 active:opacity-80",
      },
      size: { sm: "h-8 px-3", md: "h-9 px-4" },
    },
    defaultVariants: { variant: "primary", size: "md" },
  },
);

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof button> & { icon?: ReactNode };

export function Button({
  variant,
  size,
  icon,
  children,
  className,
  ...props
}: ButtonProps): JSX.Element {
  return (
    <button className={cn(button({ variant, size }), className)} {...props}>
      {icon}
      {children}
    </button>
  );
}
