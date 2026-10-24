import type { Flow, Packet } from "@/types/ipc";

/** The six dimensions `build_plan.md` Phase 3 asks for. Empty string means "any". */
export type TrafficFilters = {
  process: string;
  user: string;
  /** Remote IP or domain; substring match against both. */
  remote: string;
  /** Local or remote port; exact match. */
  port: string;
  /** Protocol label as Rust reports it (`TLS`, `DNS`, …). */
  protocol: string;
  direction: "" | "in" | "out";
};

export const EMPTY_FILTERS: TrafficFilters = {
  process: "",
  user: "",
  remote: "",
  port: "",
  protocol: "",
  direction: "",
};

/** The fields packets and flows have in common, so one predicate serves both tables. */
export type FilterableRow = Pick<
  Flow,
  "protocol" | "direction" | "remoteAddress" | "remotePort" | "remoteHost" | "processName" | "userAccount"
> & { localPort: number | null };

/** Packets carry a nullable remote address; flows always have one. */
export type FilterablePacket = Pick<
  Packet,
  | "protocol"
  | "direction"
  | "remoteAddress"
  | "remotePort"
  | "remoteHost"
  | "processName"
  | "userAccount"
  | "localPort"
>;

export function hasActiveFilters(filters: TrafficFilters): boolean {
  return Object.values(filters).some((value) => value.trim() !== "");
}

export function matchesFilters(
  row: FilterableRow | FilterablePacket,
  filters: TrafficFilters,
): boolean {
  if (!contains(row.processName, filters.process)) return false;
  if (!contains(row.userAccount, filters.user)) return false;
  if (filters.remote.trim() !== "") {
    const needle = filters.remote.trim().toLowerCase();
    const address = row.remoteAddress?.toLowerCase() ?? "";
    const host = row.remoteHost?.toLowerCase() ?? "";
    if (!address.includes(needle) && !host.includes(needle)) return false;
  }
  if (filters.port.trim() !== "") {
    const wanted = Number.parseInt(filters.port, 10);
    // A non-numeric port filter can never match, rather than matching everything.
    if (!Number.isInteger(wanted)) return false;
    if (row.remotePort !== wanted && row.localPort !== wanted) return false;
  }
  if (
    filters.protocol.trim() !== "" &&
    row.protocol.toLowerCase() !== filters.protocol.trim().toLowerCase()
  ) {
    return false;
  }
  if (filters.direction !== "" && row.direction !== filters.direction) return false;
  return true;
}

/** Protocols present in the current rows, so the select never offers a dead option. */
export function protocolOptions(rows: readonly { protocol: string }[]): string[] {
  return [...new Set(rows.map((row) => row.protocol))].sort((left, right) =>
    left.localeCompare(right),
  );
}

function contains(value: string | null, needle: string): boolean {
  if (needle.trim() === "") return true;
  return (value ?? "").toLowerCase().includes(needle.trim().toLowerCase());
}
