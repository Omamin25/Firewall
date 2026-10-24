import { useEffect } from "react";
import { Card } from "@/components/ui/card";
import { ErrorState } from "@/components/ui/error-state";
import { CaptureToolbar } from "@/features/traffic/capture-toolbar";
import { FilterBar } from "@/features/traffic/filter-bar";
import { LiveTrafficTable } from "@/features/traffic/live-traffic-table";
import { PacketInspector } from "@/features/inspector/packet-inspector";
import { hasActiveFilters } from "@/features/traffic/traffic-filters";
import { useCaptureController } from "@/features/traffic/use-capture-controller";
import { useFilteredPackets } from "@/features/traffic/use-filtered-packets";
import { useTrafficStore } from "@/store/use-traffic-store";

export function LivePage(): JSX.Element {
  const { start, stop, refreshInterfaces } = useCaptureController();
  const { rows, total, protocols } = useFilteredPackets();
  const error = useTrafficStore((state) => state.error);
  const captureState = useTrafficStore((state) => state.captureState);
  const filters = useTrafficStore((state) => state.filters);
  const setFilters = useTrafficStore((state) => state.setFilters);
  const resetFilters = useTrafficStore((state) => state.resetFilters);

  useEffect(() => {
    void refreshInterfaces();
  }, [refreshInterfaces]);

  return (
    <div className="space-y-4">
      {error ? (
        <ErrorState
          title={captureState === "error" ? "Capture engine error" : "Capture warning"}
          detail={error}
        />
      ) : null}
      <Card title="Live traffic" action={<CaptureToolbar onStart={() => void start()} onStop={() => void stop()} />}>
        <div className="space-y-3">
          <FilterBar
            filters={filters}
            onChange={setFilters}
            onReset={resetFilters}
            protocols={protocols}
            matching={rows.length}
            total={total}
          />
          <LiveTrafficTable packets={rows} filtered={hasActiveFilters(filters)} />
        </div>
      </Card>
      <PacketInspector />
    </div>
  );
}
