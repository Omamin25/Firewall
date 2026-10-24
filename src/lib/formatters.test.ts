import { describe, expect, it } from "vitest";
import {
  formatAbsoluteTime,
  formatAddressPort,
  formatBytes,
  formatCount,
  formatDuration,
  formatEndpoint,
  formatRate,
  formatRelativeTime,
  formatTime,
} from "@/lib/formatters";

describe("formatBytes", () => {
  it("formats sub-kilobyte values in bytes", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("formats kilobytes with one decimal", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
  });

  it("scales up to larger units", () => {
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
    expect(formatBytes(3 * 1024 ** 3)).toBe("3.0 GB");
  });

  it("guards against invalid input", () => {
    expect(formatBytes(-1)).toBe("—");
    expect(formatBytes(Number.NaN)).toBe("—");
  });
});

describe("formatRate", () => {
  it("suffixes a per-second unit", () => {
    expect(formatRate(2048)).toBe("2.0 KB/s");
  });

  it("reports zero for idle links", () => {
    expect(formatRate(0)).toBe("0 B/s");
  });
});

describe("formatTime", () => {
  it("includes milliseconds", () => {
    expect(formatTime("2026-08-01T10:20:30.450Z")).toMatch(/\.450$/);
  });

  it("falls back for unparseable timestamps", () => {
    expect(formatTime("not-a-date")).toBe("—");
  });
});

describe("formatEndpoint", () => {
  it("prefers the resolved host name", () => {
    expect(formatEndpoint("example.com", "93.184.216.34")).toBe("example.com");
  });

  it("falls back to the address, then to a dash", () => {
    expect(formatEndpoint(null, "93.184.216.34")).toBe("93.184.216.34");
    expect(formatEndpoint(null, null)).toBe("—");
  });
});

describe("formatAddressPort", () => {
  it("joins address and port", () => {
    expect(formatAddressPort("192.168.1.20", 443)).toBe("192.168.1.20:443");
  });

  it("omits the port when the protocol has none", () => {
    expect(formatAddressPort("192.168.1.20", null)).toBe("192.168.1.20");
  });
});

describe("formatCount", () => {
  it("groups thousands", () => {
    expect(formatCount(12345)).toBe((12345).toLocaleString());
  });
});

describe("formatDuration", () => {
  it("keeps sub-second flows in milliseconds", () => {
    expect(formatDuration(0)).toBe("0ms");
    expect(formatDuration(450)).toBe("450ms");
  });

  it("shows one decimal under ten seconds", () => {
    expect(formatDuration(1400)).toBe("1.4s");
    expect(formatDuration(9900)).toBe("9.9s");
  });

  it("drops the decimal from ten seconds up", () => {
    expect(formatDuration(10_000)).toBe("10s");
    expect(formatDuration(59_000)).toBe("59s");
  });

  it("splits minutes and hours with padded remainders", () => {
    expect(formatDuration(125_000)).toBe("2m 05s");
    expect(formatDuration(4_320_000)).toBe("1h 12m");
  });

  it("guards against invalid input", () => {
    expect(formatDuration(-1)).toBe("—");
    expect(formatDuration(Number.NaN)).toBe("—");
  });
});

describe("formatRelativeTime", () => {
  const now = new Date("2026-08-01T12:00:00.000Z").getTime();

  it("describes recent activity in seconds", () => {
    expect(formatRelativeTime("2026-08-01T11:59:59.900Z", now)).toBe("now");
    expect(formatRelativeTime("2026-08-01T11:59:45.000Z", now)).toBe("15s ago");
  });

  it("steps up through minutes, hours, and days", () => {
    expect(formatRelativeTime("2026-08-01T11:30:00.000Z", now)).toBe("30m ago");
    expect(formatRelativeTime("2026-08-01T09:00:00.000Z", now)).toBe("3h ago");
    expect(formatRelativeTime("2026-07-29T12:00:00.000Z", now)).toBe("3d ago");
  });

  it("never reports a negative age for a clock-skewed timestamp", () => {
    expect(formatRelativeTime("2026-08-01T12:00:05.000Z", now)).toBe("now");
  });

  it("falls back for unparseable timestamps", () => {
    expect(formatRelativeTime("not-a-date", now)).toBe("—");
  });
});

describe("formatAbsoluteTime", () => {
  it("falls back for unparseable timestamps", () => {
    expect(formatAbsoluteTime("not-a-date")).toBe("—");
  });

  it("renders a locale date-time for a valid timestamp", () => {
    expect(formatAbsoluteTime("2026-08-01T10:20:30.000Z")).toBe(
      new Date("2026-08-01T10:20:30.000Z").toLocaleString([], { hour12: false }),
    );
  });
});
