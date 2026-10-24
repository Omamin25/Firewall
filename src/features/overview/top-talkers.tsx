import { Users } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { EmptyState } from "@/components/ui/empty-state";
import { cn } from "@/lib/cn";
import { formatBytes, formatCount } from "@/lib/formatters";
import type { TalkerRow } from "@/features/overview/overview-metrics";

export type TopTalkersProps = {
  byProcess: TalkerRow[];
  byHost: TalkerRow[];
};

/** Who is actually using the network, from both directions the user thinks about it:
 *  which program, and which remote host. */
export function TopTalkers({ byProcess, byHost }: TopTalkersProps): JSX.Element {
  const empty = byProcess.length === 0 && byHost.length === 0;

  return (
    <Card title="Top talkers">
      {empty ? (
        <EmptyState
          icon={<Users aria-hidden size={20} />}
          title="No connections to rank yet"
          description="Once traffic is flowing, the busiest processes and remote hosts appear here."
        />
      ) : (
        <div className="grid gap-6 md:grid-cols-2">
          <TalkerList title="By process" rows={byProcess} />
          <TalkerList title="By remote host" rows={byHost} />
        </div>
      )}
    </Card>
  );
}

function TalkerList({ title, rows }: { title: string; rows: TalkerRow[] }): JSX.Element {
  const max = rows.reduce((peak, row) => Math.max(peak, row.bytes), 0);

  return (
    <section>
      <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-foreground-muted">
        {title}
      </h3>
      {rows.length === 0 ? (
        <p className="text-sm text-foreground-subtle">Nothing yet.</p>
      ) : (
        <ol className="space-y-2">
          {rows.map((row) => (
            <li key={row.key}>
              <div className="flex items-baseline justify-between gap-3">
                <span className="flex min-w-0 items-center gap-2">
                  <span className="truncate text-sm" title={row.detail ?? row.label}>
                    {row.label}
                  </span>
                  {row.blocked ? <Badge variant="danger">Blocked</Badge> : null}
                </span>
                <span
                  className="shrink-0 font-mono text-xs text-foreground-muted"
                  title={`${row.bytes} bytes over ${row.flows} connections`}
                >
                  {formatBytes(row.bytes)}
                </span>
              </div>
              <div className="mt-1 flex items-center gap-2">
                {/* Width is data, not a themed value — the track and fill colours are tokens. */}
                <div className="h-1 min-w-0 flex-1 overflow-hidden rounded-full bg-surface-muted">
                  <div
                    className={cn("h-full rounded-full", row.blocked ? "bg-danger-strong" : "bg-primary-strong")}
                    style={{ width: `${max > 0 ? Math.round((row.bytes / max) * 100) : 0}%` }}
                  />
                </div>
                <span className="shrink-0 text-xs text-foreground-subtle">
                  {formatCount(row.flows)} {row.flows === 1 ? "conn" : "conns"}
                </span>
              </div>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
