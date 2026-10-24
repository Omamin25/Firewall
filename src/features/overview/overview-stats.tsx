import { Activity, Gauge, Network, ShieldX } from "lucide-react";
import { StatTile } from "@/components/data/stat-tile";
import { formatBytes, formatCount, formatRate } from "@/lib/formatters";
import {
  seriesPeak,
  seriesSpanSeconds,
  type ThroughputSample,
} from "@/features/overview/throughput-series";
import type { ConnectionCounts } from "@/features/overview/overview-metrics";
import type { EnforcementStatus, StreamStats } from "@/types/ipc";

export type OverviewStatsProps = {
  stats: StreamStats;
  counts: ConnectionCounts;
  throughput: readonly ThroughputSample[];
  enforcement: EnforcementStatus | null;
};

/** The four numbers `build_plan.md` Phase 5 asks for up front: how much is moving, how
 *  many conversations are open, how much is being stopped, and how much was seen. */
export function OverviewStats({
  stats,
  counts,
  throughput,
  enforcement,
}: OverviewStatsProps): JSX.Element {
  const span = seriesSpanSeconds(throughput);
  const peak = seriesPeak(throughput);
  const dropped = enforcement?.blockedPackets ?? 0;

  return (
    <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <StatTile
        label="Throughput"
        icon={<Gauge aria-hidden size={13} />}
        value={formatRate(stats.bytesPerSecond)}
        delta={
          span > 0 ? `Peak ${formatRate(peak)} over the last ${span}s` : "Waiting for samples"
        }
        hint="Bytes per second across every captured interface."
      />
      <StatTile
        label="Active connections"
        icon={<Network aria-hidden size={13} />}
        value={formatCount(counts.active)}
        delta={`${formatCount(counts.total)} tracked · ${formatCount(counts.attributed)} attributed`}
        hint="Connections still exchanging packets, out of everything in the live table."
      />
      <StatTile
        label="Blocked connections"
        icon={<ShieldX aria-hidden size={13} />}
        value={formatCount(counts.blocked)}
        accent={counts.blocked > 0 ? "danger" : "neutral"}
        delta={`${formatCount(dropped)} packets dropped`}
        hint="Connections a rule has dropped packets from since launch."
      />
      <StatTile
        label="Packets captured"
        icon={<Activity aria-hidden size={13} />}
        value={formatCount(stats.packetsCaptured)}
        accent={stats.packetsDropped > 0 ? "warning" : "neutral"}
        delta={`${formatBytes(stats.bytesCaptured)} · ${formatCount(stats.packetsDropped)} dropped`}
        hint="Frames decoded by the capture engine. Dropped frames mean the queue overflowed."
      />
    </div>
  );
}
