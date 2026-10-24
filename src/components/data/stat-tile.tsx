import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** Which semantic token tints the value. `neutral` leaves it as primary text — the
 *  default, because `ui_rules.md` §1 keeps the overview calm. */
export type StatAccent = "neutral" | "success" | "warning" | "danger" | "primary";

const ACCENT_CLASSES: Record<StatAccent, string> = {
  neutral: "text-foreground",
  success: "text-success-strong",
  warning: "text-warning-strong",
  danger: "text-danger-strong",
  primary: "text-primary-strong",
};

export type StatTileProps = {
  label: string;
  /** Already formatted — the tile does no number formatting of its own. */
  value: string;
  /** Secondary line under the value: a delta, a rate, or a qualifier. */
  delta?: string;
  accent?: StatAccent;
  icon?: ReactNode;
  /** Longer explanation shown on hover. */
  hint?: string;
};

/** Overview KPI tile. Card surface + hairline border, big mono number. */
export function StatTile({
  label,
  value,
  delta,
  accent = "neutral",
  icon,
  hint,
}: StatTileProps): JSX.Element {
  return (
    <div
      className="rounded-lg border border-border bg-surface p-4 shadow-sm"
      title={hint}
    >
      <div className="flex items-center gap-2 text-xs font-medium text-foreground-muted">
        {icon}
        <span>{label}</span>
      </div>
      <p className={cn("mt-2 font-mono text-display font-semibold tracking-tight", ACCENT_CLASSES[accent])}>
        {value}
      </p>
      {delta ? <p className="mt-1 text-xs text-foreground-subtle">{delta}</p> : null}
    </div>
  );
}
