import { create } from "zustand";
import { appendSample, type ThroughputSample } from "@/features/overview/throughput-series";
import { EMPTY_FILTERS, type TrafficFilters } from "@/features/traffic/traffic-filters";
import type { CaptureInterface, Packet, StreamMessage, StreamStats } from "@/types/ipc";

/** Ring cap for the live table. Older packets fall out of the UI buffer; the Rust
 *  ring keeps a deeper history for the inspector. */
const MAX_PACKETS = 5000;

const EMPTY_STATS: StreamStats = {
  packetsCaptured: 0,
  bytesCaptured: 0,
  packetsDropped: 0,
  packetsPerSecond: 0,
  bytesPerSecond: 0,
  activeFlows: 0,
};

export type CaptureState = "idle" | "starting" | "running" | "error";

export type TrafficState = {
  packets: Packet[];
  stats: StreamStats;
  /** Rolling throughput readings for the overview sparkline; sampled at 1 Hz. */
  throughput: ThroughputSample[];
  captureState: CaptureState;
  error: string | null;
  interfaces: CaptureInterface[];
  /** Frozen view: batches still arrive but are not rendered, so a user can inspect. */
  paused: boolean;
  selectedPacketId: number | null;
  subscribed: boolean;
  filters: TrafficFilters;
  apply: (message: StreamMessage) => void;
  setCaptureState: (state: CaptureState) => void;
  setError: (error: string | null) => void;
  setInterfaces: (interfaces: CaptureInterface[]) => void;
  setPaused: (paused: boolean) => void;
  selectPacket: (id: number | null) => void;
  markSubscribed: () => void;
  setFilters: (patch: Partial<TrafficFilters>) => void;
  resetFilters: () => void;
  clear: () => void;
};

export const useTrafficStore = create<TrafficState>((set) => ({
  packets: [],
  stats: EMPTY_STATS,
  throughput: [],
  captureState: "idle",
  error: null,
  interfaces: [],
  paused: false,
  selectedPacketId: null,
  subscribed: false,
  filters: EMPTY_FILTERS,

  apply: (message) =>
    set((state) => {
      switch (message.type) {
        case "packets":
          // While paused the table keeps its current contents; stats keep flowing.
          if (state.paused) return {};
          return { packets: [...message.data].reverse().concat(state.packets).slice(0, MAX_PACKETS) };
        case "stats":
          // The series keeps its identity between samples, so the sparkline repaints once
          // a second rather than on every batch.
          return {
            stats: message.data,
            throughput: appendSample(state.throughput, message.data.bytesPerSecond, Date.now()),
          };
        case "captureError":
          return { error: message.data.message, captureState: "error" };
        // Flow diffs belong to the flow store.
        default:
          return {};
      }
    }),

  setCaptureState: (captureState) => set({ captureState }),
  setError: (error) => set({ error }),
  setInterfaces: (interfaces) => set({ interfaces }),
  setPaused: (paused) => set({ paused }),
  selectPacket: (selectedPacketId) => set({ selectedPacketId }),
  markSubscribed: () => set({ subscribed: true }),
  setFilters: (patch) => set((state) => ({ filters: { ...state.filters, ...patch } })),
  resetFilters: () => set({ filters: EMPTY_FILTERS }),
  clear: () => set({ packets: [], selectedPacketId: null }),
}));
