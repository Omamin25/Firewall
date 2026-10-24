import { useMemo } from "react";
import { matchesFilters, protocolOptions } from "@/features/traffic/traffic-filters";
import { useFlowStore } from "@/store/use-flow-store";
import type { Flow } from "@/types/ipc";

export type FilteredFlows = {
  /** Rows passing the current filters, newest activity first. */
  rows: Flow[];
  total: number;
  protocols: string[];
};

/** Derives the Connections table's rows from the live flow store. Called once per page
 *  render so the filter bar and the table agree on the counts without filtering twice. */
export function useFilteredFlows(): FilteredFlows {
  const flows = useFlowStore((state) => state.flows);
  const filters = useFlowStore((state) => state.filters);

  return useMemo(() => {
    const rows = flows
      .filter((flow) => matchesFilters(flow, filters))
      .sort((left, right) => right.lastSeen.localeCompare(left.lastSeen));
    return { rows, total: flows.length, protocols: protocolOptions(flows) };
  }, [flows, filters]);
}
