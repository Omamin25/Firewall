import type { Flow, ProtocolToken } from "@/types/ipc";

/** A row in one of the top-talker lists. `label` is what the user reads; `bytes` orders it. */
export type TalkerRow = {
  /** Stable React key — the grouping value itself. */
  key: string;
  label: string;
  /** Secondary line: the process path, or the address behind a hostname. */
  detail: string | null;
  bytes: number;
  flows: number;
  /** True when any of the grouped connections was dropped by a rule. */
  blocked: boolean;
};

export type ProtocolSlice = {
  protocol: string;
  token: ProtocolToken;
  bytes: number;
  flows: number;
  /** Fraction of total bytes, 0–1. */
  share: number;
};

export type ConnectionCounts = {
  total: number;
  active: number;
  blocked: number;
  /** Connections with a resolved owning process. */
  attributed: number;
  bytes: number;
};

const UNATTRIBUTED = "Unattributed";

/** Counts for the overview tiles. Derived from the live flow store, which is a view of
 *  what Rust already decided — no attribution or matching happens here. */
export function connectionCounts(flows: readonly Flow[]): ConnectionCounts {
  let active = 0;
  let blocked = 0;
  let attributed = 0;
  let bytes = 0;
  for (const flow of flows) {
    if (flow.active) active += 1;
    if (flow.blocked) blocked += 1;
    if (flow.processName !== null || flow.pid !== null) attributed += 1;
    bytes += flow.bytesSent + flow.bytesRecv;
  }
  return { total: flows.length, active, blocked, attributed, bytes };
}

/** Busiest processes by total bytes. Connections with no resolved owner are grouped
 *  together rather than dropped, so the totals still add up. */
export function topTalkersByProcess(flows: readonly Flow[], limit = 5): TalkerRow[] {
  return rank(
    flows,
    (flow) => flow.processName ?? (flow.pid === null ? UNATTRIBUTED : `PID ${flow.pid}`),
    (flow) => flow.processPath,
    limit,
  );
}

/** Busiest remote endpoints, keyed by hostname when one is known and address otherwise. */
export function topTalkersByHost(flows: readonly Flow[], limit = 5): TalkerRow[] {
  return rank(
    flows,
    (flow) => flow.remoteHost ?? flow.remoteAddress,
    (flow) => (flow.remoteHost === null ? null : flow.remoteAddress),
    limit,
  );
}

/** Share of traffic per L7 protocol, largest first. Tokens come from Rust so the colours
 *  match the badges in every table (`ui_rules.md` §6). */
export function protocolBreakdown(flows: readonly Flow[]): ProtocolSlice[] {
  const groups = new Map<string, { token: ProtocolToken; bytes: number; flows: number }>();
  let total = 0;
  for (const flow of flows) {
    const bytes = flow.bytesSent + flow.bytesRecv;
    total += bytes;
    const group = groups.get(flow.protocol);
    if (group === undefined) {
      groups.set(flow.protocol, { token: flow.protocolToken, bytes, flows: 1 });
    } else {
      group.bytes += bytes;
      group.flows += 1;
    }
  }
  return [...groups.entries()]
    .map(([protocol, group]) => ({
      protocol,
      token: group.token,
      bytes: group.bytes,
      flows: group.flows,
      // With no bytes yet, connection count is the only honest denominator.
      share: total > 0 ? group.bytes / total : group.flows / Math.max(1, flows.length),
    }))
    .sort((left, right) => right.share - left.share || left.protocol.localeCompare(right.protocol));
}

function rank(
  flows: readonly Flow[],
  keyOf: (flow: Flow) => string,
  detailOf: (flow: Flow) => string | null,
  limit: number,
): TalkerRow[] {
  const groups = new Map<string, TalkerRow>();
  for (const flow of flows) {
    const key = keyOf(flow);
    const bytes = flow.bytesSent + flow.bytesRecv;
    const existing = groups.get(key);
    if (existing === undefined) {
      groups.set(key, {
        key,
        label: key,
        detail: detailOf(flow),
        bytes,
        flows: 1,
        blocked: flow.blocked,
      });
    } else {
      existing.bytes += bytes;
      existing.flows += 1;
      existing.blocked = existing.blocked || flow.blocked;
      existing.detail = existing.detail ?? detailOf(flow);
    }
  }
  return [...groups.values()]
    .sort((left, right) => right.bytes - left.bytes || left.label.localeCompare(right.label))
    .slice(0, limit);
}
