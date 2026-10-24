import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export type KeyValueRowProps = {
  label: string;
  value: ReactNode;
  mono?: boolean;
};

/** Inspector field row: muted label left, value right (`ui_rules.md` §7). */
export function KeyValueRow({ label, value, mono = false }: KeyValueRowProps): JSX.Element {
  return (
    <div className="flex items-baseline justify-between gap-4 py-1">
      <dt className="shrink-0 text-xs text-foreground-muted">{label}</dt>
      <dd className={cn("min-w-0 break-all text-right text-sm", mono && "font-mono text-xs")}>
        {value}
      </dd>
    </div>
  );
}
