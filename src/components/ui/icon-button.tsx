import { cva, type VariantProps } from "class-variance-authority";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

const iconButton = cva(
  "inline-flex items-center justify-center rounded-md transition-opacity duration-fast ease-enter focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50",
  {
    variants: {
      variant: {
        ghost: "text-foreground-muted hover:bg-surface-muted hover:text-foreground",
        outline: "border border-border-strong text-foreground hover:bg-surface-muted",
        destructive: "text-danger-strong hover:bg-danger/10",
      },
      // Both sizes clear the 32px hit target from `ui_rules.md` §10.
      size: { sm: "size-8", md: "size-9" },
    },
    defaultVariants: { variant: "ghost", size: "sm" },
  },
);

export type IconButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> &
  VariantProps<typeof iconButton> & {
    icon: ReactNode;
    /** Accessible name, also used as the hover tooltip for this icon-only control. */
    label: string;
  };

export function IconButton({
  icon,
  label,
  variant,
  size,
  className,
  ...props
}: IconButtonProps): JSX.Element {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={cn(iconButton({ variant, size }), className)}
      {...props}
    >
      {icon}
    </button>
  );
}
