import { Database } from "lucide-react";
import { formatAbsoluteTime, formatCount } from "@/lib/formatters";
import type { FlowHistorySummary } from "@/types/ipc";

export type FlowHistoryStripProps = { history: FlowHistorySummary | null };

/** Makes flow persistence and pruning observable: how many aggregated flows are stored,
 *  the window they span, and the retention policy enforcing it. */
export function FlowHistoryStrip({ history }: FlowHistoryStripProps): JSX.Element {
  if (!history) {
    return (
      <p className="text-xs text-foreground-muted">Loading stored connection history…</p>
    );
  }

  return (
    <dl className="flex flex-wrap items-center gap-x-6 gap-y-1 text-xs text-foreground-muted">
      <div className="flex items-center gap-1.5">
        <Database aria-hidden size={13} />
        <span>Stored history</span>
      </div>
      <Stat label="Flows" value={formatCount(history.storedFlows)} />
      <Stat
        label="Retention"
        value={`${history.retentionHours}h · max ${formatCount(history.maxRows)} rows`}
      />
      {history.oldestSeen ? (
        <Stat label="Oldest" value={formatAbsoluteTime(history.oldestSeen)} />
      ) : null}
      {history.newestSeen ? (
        <Stat label="Newest" value={formatAbsoluteTime(history.newestSeen)} />
      ) : null}
    </dl>
  );
}

type StatProps = { label: string; value: string };

function Stat({ label, value }: StatProps): JSX.Element {
  return (
    <div className="flex items-baseline gap-1.5">
      <dt>{label}</dt>
      <dd className="font-mono text-foreground">{value}</dd>
    </div>
  );
}
