import { useEffect, useMemo } from "react";
import { ErrorState } from "@/components/ui/error-state";
import {
  connectionCounts,
  protocolBreakdown,
  topTalkersByHost,
  topTalkersByProcess,
} from "@/features/overview/overview-metrics";
import { OverviewStats } from "@/features/overview/overview-stats";
import { ProtocolBreakdown } from "@/features/overview/protocol-breakdown";
import { ThroughputCard } from "@/features/overview/throughput-card";
import { TopTalkers } from "@/features/overview/top-talkers";
import { useCaptureController } from "@/features/traffic/use-capture-controller";
import { useFlowStore } from "@/store/use-flow-store";
import { useRulesStore } from "@/store/use-rules-store";
import { useTrafficStore } from "@/store/use-traffic-store";

/** Blocked-packet counters move without any rule changing, so the engine state is polled
 *  while this page is open. */
const ENFORCEMENT_REFRESH_MS = 5_000;

export function OverviewPage(): JSX.Element {
  // Registers the stream channel and seeds the flow table if this is the first view opened.
  useCaptureController();
  const flows = useFlowStore((state) => state.flows);
  const flowError = useFlowStore((state) => state.error);
  const stats = useTrafficStore((state) => state.stats);
  const throughput = useTrafficStore((state) => state.throughput);
  const captureState = useTrafficStore((state) => state.captureState);
  const captureError = useTrafficStore((state) => state.error);
  const enforcement = useRulesStore((state) => state.enforcement);
  const refreshEnforcement = useRulesStore((state) => state.refreshEnforcement);

  useEffect(() => {
    void refreshEnforcement();
    const timer = window.setInterval(() => void refreshEnforcement(), ENFORCEMENT_REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [refreshEnforcement]);

  // One pass over the flow list per render, shared by every panel below.
  const metrics = useMemo(
    () => ({
      counts: connectionCounts(flows),
      byProcess: topTalkersByProcess(flows),
      byHost: topTalkersByHost(flows),
      protocols: protocolBreakdown(flows),
    }),
    [flows],
  );

  return (
    <div className="space-y-4">
      {captureError ? (
        <ErrorState
          title={captureState === "error" ? "Capture engine error" : "Capture warning"}
          detail={captureError}
        />
      ) : null}
      {flowError ? <ErrorState title="Connections unavailable" detail={flowError} /> : null}
      {enforcement?.error ? (
        <ErrorState title="Enforcement unavailable" detail={enforcement.error} />
      ) : null}

      <OverviewStats
        stats={stats}
        counts={metrics.counts}
        throughput={throughput}
        enforcement={enforcement}
      />
      <ThroughputCard series={throughput} capturing={captureState === "running"} />
      <div className="grid gap-4 xl:grid-cols-2">
        <TopTalkers byProcess={metrics.byProcess} byHost={metrics.byHost} />
        <ProtocolBreakdown slices={metrics.protocols} />
      </div>
    </div>
  );
}
