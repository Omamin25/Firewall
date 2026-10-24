import { describe, expect, it } from "vitest";
import {
  connectionCounts,
  protocolBreakdown,
  topTalkersByHost,
  topTalkersByProcess,
} from "@/features/overview/overview-metrics";
import type { Flow } from "@/types/ipc";

function flow(patch: Partial<Flow> = {}): Flow {
  return {
    id: 1,
    firstSeen: "2026-08-01T10:00:00Z",
    lastSeen: "2026-08-01T10:00:05Z",
    direction: "out",
    protocol: "TLS",
    protocolToken: "tls",
    transport: "tcp",
    localAddress: "192.168.1.20",
    localPort: 51_000,
    remoteAddress: "93.184.216.34",
    remotePort: 443,
    remoteHost: "example.com",
    bytesSent: 100,
    bytesRecv: 100,
    packets: 4,
    durationMs: 5_000,
    active: true,
    pid: 4_242,
    processName: "browser.exe",
    processPath: "C:\\Apps\\browser.exe",
    userAccount: "OMEN\\Vraj",
    userSid: "S-1-5-21-7-1001",
    blocked: false,
    matchedRuleId: null,
    matchedRuleName: null,
    ...patch,
  };
}

describe("connectionCounts", () => {
  it("counts live, blocked and attributed connections", () => {
    const counts = connectionCounts([
      flow({ id: 1 }),
      flow({ id: 2, active: false }),
      flow({ id: 3, blocked: true }),
      flow({ id: 4, pid: null, processName: null, processPath: null }),
    ]);
    expect(counts).toEqual({ total: 4, active: 3, blocked: 1, attributed: 3, bytes: 800 });
  });

  it("is all zeroes for an empty table", () => {
    expect(connectionCounts([])).toEqual({
      total: 0,
      active: 0,
      blocked: 0,
      attributed: 0,
      bytes: 0,
    });
  });
});

describe("topTalkersByProcess", () => {
  it("sums bytes per process and orders by volume", () => {
    const rows = topTalkersByProcess([
      flow({ id: 1, processName: "browser.exe", bytesSent: 500, bytesRecv: 0 }),
      flow({ id: 2, processName: "browser.exe", bytesSent: 100, bytesRecv: 0 }),
      flow({ id: 3, processName: "updater.exe", bytesSent: 900, bytesRecv: 0 }),
    ]);
    expect(rows.map((row) => [row.label, row.bytes, row.flows])).toEqual([
      ["updater.exe", 900, 1],
      ["browser.exe", 600, 2],
    ]);
  });

  it("groups connections with no owner instead of hiding them", () => {
    const rows = topTalkersByProcess([
      flow({ id: 1, pid: null, processName: null, processPath: null, bytesSent: 10, bytesRecv: 0 }),
    ]);
    expect(rows[0]?.label).toBe("Unattributed");
  });

  it("falls back to the PID when only the owner id is known", () => {
    const rows = topTalkersByProcess([flow({ id: 1, processName: null, pid: 77 })]);
    expect(rows[0]?.label).toBe("PID 77");
  });

  it("marks a group blocked when any of its connections was dropped", () => {
    const rows = topTalkersByProcess([
      flow({ id: 1, blocked: false }),
      flow({ id: 2, blocked: true }),
    ]);
    expect(rows[0]?.blocked).toBe(true);
  });

  it("honours the row limit", () => {
    const flows = Array.from({ length: 9 }, (_, index) =>
      flow({ id: index, processName: `p${index}.exe`, bytesSent: index, bytesRecv: 0 }),
    );
    expect(topTalkersByProcess(flows, 3)).toHaveLength(3);
  });
});

describe("topTalkersByHost", () => {
  it("prefers the hostname and keeps the address as detail", () => {
    const rows = topTalkersByHost([flow({ id: 1 })]);
    expect(rows[0]?.label).toBe("example.com");
    expect(rows[0]?.detail).toBe("93.184.216.34");
  });

  it("falls back to the address when no name was resolved", () => {
    const rows = topTalkersByHost([flow({ id: 1, remoteHost: null })]);
    expect(rows[0]?.label).toBe("93.184.216.34");
    expect(rows[0]?.detail).toBeNull();
  });
});

describe("protocolBreakdown", () => {
  it("shares bytes across protocols, largest first", () => {
    const slices = protocolBreakdown([
      flow({ id: 1, protocol: "TLS", protocolToken: "tls", bytesSent: 300, bytesRecv: 0 }),
      flow({ id: 2, protocol: "DNS", protocolToken: "dns", bytesSent: 100, bytesRecv: 0 }),
    ]);
    expect(slices.map((slice) => [slice.protocol, slice.share])).toEqual([
      ["TLS", 0.75],
      ["DNS", 0.25],
    ]);
    expect(slices[0]?.token).toBe("tls");
  });

  it("falls back to connection share before any bytes are counted", () => {
    const slices = protocolBreakdown([
      flow({ id: 1, protocol: "ARP", protocolToken: "other", bytesSent: 0, bytesRecv: 0 }),
      flow({ id: 2, protocol: "DNS", protocolToken: "dns", bytesSent: 0, bytesRecv: 0 }),
    ]);
    expect(slices.every((slice) => slice.share === 0.5)).toBe(true);
  });

  it("returns nothing for an empty table", () => {
    expect(protocolBreakdown([])).toEqual([]);
  });
});
