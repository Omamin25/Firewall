import { useMemo } from "react";
import { matchesFilters, protocolOptions } from "@/features/traffic/traffic-filters";
import { useTrafficStore } from "@/store/use-traffic-store";
import type { Packet } from "@/types/ipc";

export type FilteredPackets = {
  /** Rows passing the current filters, newest first (the store already prepends). */
  rows: Packet[];
  total: number;
  protocols: string[];
};

/** Derives the Live table's rows. Called once per page render so the filter bar and the
 *  table agree on the counts without filtering twice. */
export function useFilteredPackets(): FilteredPackets {
  const packets = useTrafficStore((state) => state.packets);
  const filters = useTrafficStore((state) => state.filters);

  return useMemo(
    () => ({
      rows: packets.filter((packet) => matchesFilters(packet, filters)),
      total: packets.length,
      protocols: protocolOptions(packets),
    }),
    [packets, filters],
  );
}
