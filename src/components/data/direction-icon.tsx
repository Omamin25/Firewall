import { ArrowDown, ArrowUp } from "lucide-react";

export type DirectionIconProps = { direction: "in" | "out" };

/** Arrow plus text label — direction is never conveyed by the icon alone. */
export function DirectionIcon({ direction }: DirectionIconProps): JSX.Element {
  const outbound = direction === "out";
  return (
    <span className="inline-flex items-center gap-1 text-foreground-muted">
      {outbound ? (
        <ArrowUp aria-hidden className="size-3.5" />
      ) : (
        <ArrowDown aria-hidden className="size-3.5" />
      )}
      <span className="text-xs">{outbound ? "Out" : "In"}</span>
    </span>
  );
}
