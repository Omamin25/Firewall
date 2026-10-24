import { useEffect, useState } from "react";
import { Card } from "@/components/ui/card";
import { ErrorState } from "@/components/ui/error-state";
import { CreateRuleFromFlow } from "@/features/rules/create-rule-from-flow";
import { ConnectionsTable, type MenuAnchor } from "@/features/traffic/connections-table";
import { FilterBar } from "@/features/traffic/filter-bar";
import { FlowHistoryStrip } from "@/features/traffic/flow-history-strip";
import { hasActiveFilters } from "@/features/traffic/traffic-filters";
import { useCaptureController } from "@/features/traffic/use-capture-controller";
import { useFilteredFlows } from "@/features/traffic/use-filtered-flows";
import { getFlowHistorySummary } from "@/lib/ipc";
import { useFlowStore } from "@/store/use-flow-store";
import type { Flow } from "@/types/ipc";

/** How often the persisted-history strip is refreshed, so pruning is visible. */
const HISTORY_REFRESH_MS = 15_000;

export function ConnectionsPage(): JSX.Element {
  // Registers the stream channel if the user lands here before the Live view.
  useCaptureController();
  const { rows, total, protocols } = useFilteredFlows();
  const filters = useFlowStore((state) => state.filters);
  const setFilters = useFlowStore((state) => state.setFilters);
  const resetFilters = useFlowStore((state) => state.resetFilters);
  const setHistory = useFlowStore((state) => state.setHistory);
  const setError = useFlowStore((state) => state.setError);
  const history = useFlowStore((state) => state.history);
  const loading = useFlowStore((state) => state.loading);
  const error = useFlowStore((state) => state.error);
  const [rowMenu, setRowMenu] = useState<{ flow: Flow; anchor: MenuAnchor } | null>(null);

  useEffect(() => {
    let active = true;
    const load = (): void => {
      getFlowHistorySummary()
        .then((summary) => {
          if (active) setHistory(summary);
        })
        .catch((cause: unknown) => {
          if (active) setError(errorMessage(cause));
        });
    };
    load();
    const timer = window.setInterval(load, HISTORY_REFRESH_MS);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [setHistory, setError]);

  return (
    <div className="space-y-4">
      {error ? <ErrorState title="Connections unavailable" detail={error} /> : null}
      <Card title="Connections" action={<FlowHistoryStrip history={history} />}>
        <div className="space-y-3">
          <FilterBar
            filters={filters}
            onChange={setFilters}
            onReset={resetFilters}
            protocols={protocols}
            matching={rows.length}
            total={total}
          />
          <ConnectionsTable
            flows={rows}
            loading={loading}
            filtered={hasActiveFilters(filters)}
            onRowMenu={(flow, anchor) => setRowMenu({ flow, anchor })}
          />
        </div>
      </Card>
      {rowMenu ? (
        <CreateRuleFromFlow
          flow={rowMenu.flow}
          anchor={rowMenu.anchor}
          onClose={() => setRowMenu(null)}
        />
      ) : null}
    </div>
  );
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "An unexpected error occurred while loading connections.";
}
