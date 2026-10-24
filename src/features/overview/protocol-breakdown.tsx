import { PieChart } from "lucide-react";
import { Card } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { cn } from "@/lib/cn";
import { formatBytes, formatCount } from "@/lib/formatters";
import { PROTOCOL_FILL_CLASSES } from "@/lib/protocol-classes";
import type { ProtocolSlice } from "@/features/overview/overview-metrics";

export type ProtocolBreakdownProps = { slices: ProtocolSlice[] };

/** Share of traffic per L7 protocol. A single proportion bar plus a labelled legend reads
 *  better than a pie at this size, and the legend is what makes the colours meaningful
 *  (`ui_rules.md` §10 — the protocol legend is always labelled). */
export function ProtocolBreakdown({ slices }: ProtocolBreakdownProps): JSX.Element {
  return (
    <Card title="Protocol breakdown">
      {slices.length === 0 ? (
        <EmptyState
          icon={<PieChart aria-hidden size={20} />}
          title="Nothing classified yet"
          description="Protocols appear as soon as connections are decoded — DNS and TLS usually arrive first."
        />
      ) : (
        <div className="space-y-4">
          <div
            className="flex h-2 overflow-hidden rounded-full bg-surface-muted"
            role="img"
            aria-label={slices
              .map((slice) => `${slice.protocol} ${percent(slice.share)}`)
              .join(", ")}
          >
            {slices.map((slice) => (
              <div
                key={slice.protocol}
                className={cn(PROTOCOL_FILL_CLASSES[slice.token])}
                // Segment width is the measured share.
                style={{ width: `${Math.max(1, Math.round(slice.share * 100))}%` }}
                title={`${slice.protocol} · ${percent(slice.share)}`}
              />
            ))}
          </div>

          <ul className="grid gap-x-6 gap-y-1 sm:grid-cols-2">
            {slices.map((slice) => (
              <li key={slice.protocol} className="flex items-baseline justify-between gap-3">
                <span className="flex min-w-0 items-center gap-2">
                  <span
                    aria-hidden
                    className={cn("size-2 shrink-0 rounded-full", PROTOCOL_FILL_CLASSES[slice.token])}
                  />
                  <span className="truncate text-sm">{slice.protocol}</span>
                </span>
                <span
                  className="shrink-0 font-mono text-xs text-foreground-muted"
                  title={`${formatBytes(slice.bytes)} over ${formatCount(slice.flows)} connections`}
                >
                  {percent(slice.share)}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </Card>
  );
}

function percent(share: number): string {
  const value = share * 100;
  return value >= 1 ? `${Math.round(value)}%` : "<1%";
}
