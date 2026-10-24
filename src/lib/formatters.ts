const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/** Human-readable byte size. Raw values stay available via `title` attributes in cells. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < BYTE_UNITS.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(1)} ${BYTE_UNITS[unitIndex]}`;
}

/** Throughput label for the stats strip. */
export function formatRate(bytesPerSecond: number): string {
  if (!Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) return "0 B/s";
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** Wall-clock time with millisecond precision — packet ordering matters at this scale. */
export function formatTime(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return "—";
  const millis = String(date.getMilliseconds()).padStart(3, "0");
  return `${date.toLocaleTimeString([], { hour12: false })}.${millis}`;
}

/** `host` when a name is known, otherwise the bare address. */
export function formatEndpoint(host: string | null, address: string | null): string {
  return host ?? address ?? "—";
}

/** `address:port`, omitting the port when the protocol has none (ICMP/ARP). */
export function formatAddressPort(address: string | null, port: number | null): string {
  if (!address) return "—";
  return port === null ? address : `${address}:${port}`;
}

export function formatCount(value: number): string {
  return value.toLocaleString();
}

/** Compact flow duration: sub-second stays in ms, then `1.4s`, `2m 05s`, `1h 12m`. */
export function formatDuration(milliseconds: number): string {
  if (!Number.isFinite(milliseconds) || milliseconds < 0) return "—";
  if (milliseconds < 1000) return `${Math.round(milliseconds)}ms`;
  const totalSeconds = Math.floor(milliseconds / 1000);
  if (totalSeconds < 10) return `${(milliseconds / 1000).toFixed(1)}s`;
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${String(minutes % 60).padStart(2, "0")}m`;
}

/** Relative age of a timestamp, as `ui_rules.md` §5 asks for in table cells. */
export function formatRelativeTime(timestamp: string, now: number = Date.now()): string {
  const then = new Date(timestamp).getTime();
  if (Number.isNaN(then)) return "—";
  const seconds = Math.max(0, Math.round((now - then) / 1000));
  if (seconds < 1) return "now";
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

/** Absolute wall-clock label used in `title` attributes beside relative times. */
export function formatAbsoluteTime(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return "—";
  return date.toLocaleString([], { hour12: false });
}
