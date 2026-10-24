import { cn } from "@/lib/cn";

export type SkeletonProps = {
  /** Number of placeholder lines to draw. Ignored by the `block` variant. */
  rows?: number;
  variant?: "rows" | "block";
  className?: string;
};

/** Loading placeholder (`ui_rules.md` §9): never a bare spinner for a full view. The
 *  pulse is `motion-safe` so reduced-motion users get a static block. */
export function Skeleton({ rows = 4, variant = "rows", className }: SkeletonProps): JSX.Element {
  if (variant === "block") {
    return (
      <div
        aria-hidden
        className={cn("h-24 rounded-md bg-surface-muted motion-safe:animate-pulse", className)}
      />
    );
  }
  return (
    <div aria-hidden className={cn("space-y-2", className)}>
      {Array.from({ length: rows }, (_, row) => (
        <div key={row} className="h-6 rounded bg-surface-muted motion-safe:animate-pulse" />
      ))}
    </div>
  );
}

/** Skeleton shaped like a virtualized table body, framed the same way the real table is. */
export function TableSkeleton({ rows = 6 }: { rows?: number }): JSX.Element {
  return (
    <div className="rounded-md border border-border p-3">
      <Skeleton rows={rows} />
    </div>
  );
}
