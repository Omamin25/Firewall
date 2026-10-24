import type { ProtocolToken } from "@/types/ipc";

/** Tinted background + `-strong` label per protocol, for badges. The fill is the base
 *  token and the text is its legible pair (`ui_tokens.md` §1.1). */
export const PROTOCOL_BADGE_CLASSES: Record<ProtocolToken, string> = {
  dns: "bg-proto-dns/15 text-proto-dns-strong",
  http: "bg-proto-http/15 text-proto-http-strong",
  tls: "bg-proto-tls/15 text-proto-tls-strong",
  quic: "bg-proto-quic/15 text-proto-quic-strong",
  icmp: "bg-proto-icmp/15 text-proto-icmp-strong",
  other: "bg-proto-other/15 text-proto-other-strong",
};

/** Solid fills for chart marks and legend swatches. These sit directly on a surface with
 *  no tint, so they use the `-strong` pair to clear the 3:1 non-text threshold. */
export const PROTOCOL_FILL_CLASSES: Record<ProtocolToken, string> = {
  dns: "bg-proto-dns-strong",
  http: "bg-proto-http-strong",
  tls: "bg-proto-tls-strong",
  quic: "bg-proto-quic-strong",
  icmp: "bg-proto-icmp-strong",
  other: "bg-proto-other-strong",
};
