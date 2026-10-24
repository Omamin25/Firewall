import { create } from "zustand";
import { EMPTY_FILTERS, type TrafficFilters } from "@/features/traffic/traffic-filters";
import type { Flow, FlowHistorySummary, StreamMessage } from "@/types/ipc";

/** UI-side cap on live connections. Rust retires flows on its own slower timer; this is
 *  only a guard against an unbounded table if a batch arrives before that happens. */
const MAX_FLOWS = 4000;

export type FlowState = {
  flows: Flow[];
  history: FlowHistorySummary | null;
  filters: TrafficFilters;
  /** Cleared once the first snapshot has been loaded, so the view can skeleton. */
  loading: boolean;
  /** True once the live table has been seeded from `list_flows`, so it happens once per
   *  session no matter which view the user opens first. */
  seeded: boolean;
  error: string | null;
  apply: (message: StreamMessage) => void;
  replaceAll: (flows: Flow[]) => void;
  markSeeded: () => void;
  setHistory: (history: FlowHistorySummary | null) => void;
  setFilters: (patch: Partial<TrafficFilters>) => void;
  resetFilters: () => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
};

export const useFlowStore = create<FlowState>((set) => ({
  flows: [],
  history: null,
  filters: EMPTY_FILTERS,
  loading: true,
  seeded: false,
  error: null,

  apply: (message) =>
    set((state) => {
      switch (message.type) {
        case "flows":
          return { flows: merge(state.flows, message.data) };
        case "flowsRetired": {
          const retired = new Set(message.data);
          return { flows: state.flows.filter((flow) => !retired.has(flow.id)) };
        }
        // Packets, stats, and capture errors belong to the traffic store.
        default:
          return {};
      }
    }),

  replaceAll: (flows) => set({ flows, loading: false }),
  markSeeded: () => set({ seeded: true }),
  setHistory: (history) => set({ history }),
  setFilters: (patch) => set((state) => ({ filters: { ...state.filters, ...patch } })),
  resetFilters: () => set({ filters: EMPTY_FILTERS }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));

/** Patches changed flows in place and appends new ones — Rust sends diffs, not the
 *  whole table. */
function merge(current: Flow[], updates: Flow[]): Flow[] {
  if (updates.length === 0) return current;
  const positions = new Map(current.map((flow, position) => [flow.id, position]));
  const next = current.slice();
  for (const update of updates) {
    const position = positions.get(update.id);
    if (position === undefined) {
      positions.set(update.id, next.length);
      next.push(update);
    } else {
      next[position] = update;
    }
  }
  if (next.length <= MAX_FLOWS) return next;
  return next
    .sort((left, right) => right.lastSeen.localeCompare(left.lastSeen))
    .slice(0, MAX_FLOWS);
}
