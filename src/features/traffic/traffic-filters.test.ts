import { describe, expect, it } from "vitest";
import {
  EMPTY_FILTERS,
  hasActiveFilters,
  matchesFilters,
  protocolOptions,
  type FilterableRow,
  type TrafficFilters,
} from "@/features/traffic/traffic-filters";

const ROW: FilterableRow = {
  protocol: "TLS",
  direction: "out",
  remoteAddress: "93.184.216.34",
  remotePort: 443,
  remoteHost: "example.com",
  processName: "firefox.exe",
  userAccount: "OMEN\\Vraj",
  localPort: 51000,
};

function filters(overrides: Partial<TrafficFilters>): TrafficFilters {
  return { ...EMPTY_FILTERS, ...overrides };
}

describe("hasActiveFilters", () => {
  it("is false for the empty filter set", () => {
    expect(hasActiveFilters(EMPTY_FILTERS)).toBe(false);
  });

  it("ignores whitespace-only values", () => {
    expect(hasActiveFilters(filters({ process: "   " }))).toBe(false);
    expect(hasActiveFilters(filters({ process: "chrome" }))).toBe(true);
  });
});

describe("matchesFilters", () => {
  it("keeps every row when nothing is filtered", () => {
    expect(matchesFilters(ROW, EMPTY_FILTERS)).toBe(true);
  });

  it("matches process and user case-insensitively on a substring", () => {
    expect(matchesFilters(ROW, filters({ process: "FIRE" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ process: "chrome" }))).toBe(false);
    expect(matchesFilters(ROW, filters({ user: "vraj" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ user: "system" }))).toBe(false);
  });

  it("matches the remote filter against both the address and the domain", () => {
    expect(matchesFilters(ROW, filters({ remote: "93.184" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ remote: "example.com" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ remote: "google" }))).toBe(false);
  });

  it("matches either end of the connection on port", () => {
    expect(matchesFilters(ROW, filters({ port: "443" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ port: "51000" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ port: "80" }))).toBe(false);
  });

  it("treats a non-numeric port as matching nothing", () => {
    expect(matchesFilters(ROW, filters({ port: "https" }))).toBe(false);
  });

  it("matches the protocol label exactly, ignoring case", () => {
    expect(matchesFilters(ROW, filters({ protocol: "tls" }))).toBe(true);
    // A substring must not match: TLS and DTLS are different protocols.
    expect(matchesFilters(ROW, filters({ protocol: "TL" }))).toBe(false);
  });

  it("matches direction", () => {
    expect(matchesFilters(ROW, filters({ direction: "out" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ direction: "in" }))).toBe(false);
  });

  it("requires every active filter to match", () => {
    expect(matchesFilters(ROW, filters({ process: "firefox", port: "443" }))).toBe(true);
    expect(matchesFilters(ROW, filters({ process: "firefox", port: "80" }))).toBe(false);
  });

  it("excludes unattributed rows when a process or user filter is set", () => {
    const unattributed: FilterableRow = { ...ROW, processName: null, userAccount: null };
    expect(matchesFilters(unattributed, filters({ process: "firefox" }))).toBe(false);
    expect(matchesFilters(unattributed, EMPTY_FILTERS)).toBe(true);
  });
});

describe("protocolOptions", () => {
  it("returns each protocol once, sorted", () => {
    expect(
      protocolOptions([
        { protocol: "TLS" },
        { protocol: "DNS" },
        { protocol: "TLS" },
        { protocol: "QUIC" },
      ]),
    ).toEqual(["DNS", "QUIC", "TLS"]);
  });

  it("returns nothing for an empty table", () => {
    expect(protocolOptions([])).toEqual([]);
  });
});
