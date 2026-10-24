import { cva, type VariantProps } from "class-variance-authority";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

// The fill is the pale base token; the label is the `-strong` pair, which is what keeps
// it readable (`ui_tokens.md` §1.1).
const badge = cva(
  "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium",
  {
    variants: {
      variant: {
        success: "bg-success/15 text-success-strong",
        danger: "bg-danger/15 text-danger-strong",
        warning: "bg-warning/15 text-warning-strong",
        neutral: "bg-surface-muted text-foreground-muted",
      },
    },
    defaultVariants: { variant: "neutral" },
  },
);

export type BadgeProps = VariantProps<typeof badge> & {
  children: ReactNode;
  className?: string;
  /** Hover explanation — used when the badge summarises a longer status message. */
  title?: string;
};

export function Badge({ variant, children, className, title }: BadgeProps): JSX.Element {
  return <span className={cn(badge({ variant }), className)} title={title}>{children}</span>;
}
