import { Gauge } from "lucide-react";
import { Sparkline } from "@/components/data/sparkline";
import { Card } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { formatRate } from "@/lib/formatters";
import {
  seriesAverage,
  seriesPeak,
  seriesSpanSeconds,
  THROUGHPUT_CAPACITY,
  type ThroughputSample,
} from "@/features/overview/throughput-series";

export type ThroughputCardProps = {
  series: readonly ThroughputSample[];
  /** Distinguishes "capture is off" from "capture is on and quiet". */
  capturing: boolean;
};

/** Live throughput over the sampling window. One chart, no axes — the numbers beside it
 *  carry the precision (`ui_rules.md` §1: data-dense but calm). */
export function ThroughputCard({ series, capturing }: ThroughputCardProps): JSX.Element {
  const span = seriesSpanSeconds(series);

  return (
    <Card
      title="Throughput"
      action={
        series.length > 1 ? (
          <dl className="flex items-center gap-x-6 text-xs text-foreground-muted">
            <Stat label="Average" value={formatRate(seriesAverage(series))} />
            <Stat label="Peak" value={formatRate(seriesPeak(series))} />
            <Stat label="Window" value={`${span}s of ${THROUGHPUT_CAPACITY}s`} />
          </dl>
        ) : null
      }
    >
      {series.length > 1 ? (
        <Sparkline
          series={series}
          label={`Throughput over the last ${span} seconds, peaking at ${formatRate(seriesPeak(series))}`}
          className="h-32"
        />
      ) : (
        <EmptyState
          icon={<Gauge aria-hidden size={20} />}
          title={capturing ? "Collecting samples" : "No throughput yet"}
          description={
            capturing
              ? "The first points appear a second or two after packets start arriving."
              : "Start capture from the Live traffic view to chart throughput here."
          }
        />
      )}
    </Card>
  );
}

function Stat({ label, value }: { label: string; value: string }): JSX.Element {
  return (
    <div className="flex items-baseline gap-1.5">
      <dt>{label}</dt>
      <dd className="font-mono text-foreground">{value}</dd>
    </div>
  );
}
