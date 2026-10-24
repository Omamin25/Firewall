export type AppStatus = { isElevated: boolean; databaseReady: boolean; error: string | null };

/** Row shape for the Live Traffic table. Mirrors `ipc::dto::PacketDto`. */
export type Packet = {
  id: number;
  timestamp: string;
  direction: "in" | "out";
  protocol: string;
  /** `--proto-*` token key chosen in Rust; the UI never maps names to colors itself. */
  protocolToken: ProtocolToken;
  localAddress: string | null;
  localPort: number | null;
  remoteAddress: string | null;
  remotePort: number | null;
  remoteHost: string | null;
  summary: string;
  bytes: number;
  processName: string | null;
  userAccount: string | null;
};

export type ProtocolToken = "dns" | "http" | "tls" | "quic" | "icmp" | "other";

/** Mirrors `ipc::dto::PacketDetailDto`. */
export type PacketDetail = {
  id: number;
  timestamp: string;
  interface: string;
  direction: "in" | "out";
  protocol: string;
  protocolToken: ProtocolToken;
  frameLen: number;
  capturedLen: number;
  payloadLen: number;
  link: LinkLayer;
  network: NetworkLayer | null;
  transport: TransportLayer | null;
  application: ApplicationLayer | null;
  attribution: Attribution;
};

export type LinkLayer = { sourceMac: string; destinationMac: string; etherType: string };

export type NetworkLayer =
  | {
      type: "ipv4";
      source: string;
      destination: string;
      ttl: number;
      protocol: string;
      dontFragment: boolean;
      moreFragments: boolean;
      fragmentOffset: number;
    }
  | { type: "ipv6"; source: string; destination: string; hopLimit: number; nextHeader: string }
  | { type: "arp"; operation: string; senderIp: string; targetIp: string };

export type TransportLayer =
  | {
      type: "tcp";
      sourcePort: number;
      destinationPort: number;
      sequence: number;
      acknowledgment: number;
      flags: string[];
      window: number;
    }
  | { type: "udp"; sourcePort: number; destinationPort: number; length: number }
  | { type: "icmp"; kind: number; code: number; description: string }
  | { type: "igmp"; kind: number; description: string };

export type ApplicationLayer =
  | { type: "dns"; isResponse: boolean; questions: string[]; answers: string[] }
  | { type: "http"; method: string | null; host: string | null; path: string | null; status: number | null }
  | { type: "tls"; sni: string | null; version: string | null; handshake: string }
  | { type: "quic"; sni: string | null; longHeader: boolean }
  | { type: "dhcp"; messageType: string }
  | { type: "mdns"; isResponse: boolean; questions: string[] }
  | { type: "opaque"; hint: string };

export type Attribution = {
  pid: number | null;
  processName: string | null;
  processPath: string | null;
  userAccount: string | null;
  userSid: string | null;
};

/** Row shape for the Connections table. Mirrors `ipc::dto::FlowDto`. */
export type Flow = {
  id: number;
  firstSeen: string;
  lastSeen: string;
  direction: "in" | "out";
  /** Most specific protocol seen on the flow (e.g. `TLS`, not `TCP`). */
  protocol: string;
  protocolToken: ProtocolToken;
  transport: TransportProtocol;
  localAddress: string;
  localPort: number | null;
  remoteAddress: string;
  remotePort: number | null;
  remoteHost: string | null;
  bytesSent: number;
  bytesRecv: number;
  packets: number;
  durationMs: number;
  /** False once a FIN/RST was seen; the row stays until it ages out. */
  active: boolean;
  pid: number | null;
  processName: string | null;
  processPath: string | null;
  userAccount: string | null;
  userSid: string | null;
  /** True once the enforcement engine dropped a packet of this conversation. */
  blocked: boolean;
  matchedRuleId: number | null;
  /** Name of the rule that dropped it, so the row can say why. */
  matchedRuleName: string | null;
};

export type TransportProtocol = "tcp" | "udp" | "icmp" | "igmp";

export type RuleAction = "allow" | "block";
export type RuleDirection = "in" | "out" | "both";
export type RuleProtocol = "tcp" | "udp" | "icmp" | "any";

/** Mirrors `ipc::dto::RuleDto`. */
export type Rule = {
  id: number;
  name: string;
  action: RuleAction;
  direction: RuleDirection;
  protocol: RuleProtocol;
  /** Exact address or CIDR block; null matches any remote. */
  remoteIp: string | null;
  remotePort: number | null;
  localPort: number | null;
  /** Full image path, or a bare file name to match the executable alone. */
  processPath: string | null;
  userSid: string | null;
  enabled: boolean;
  /** Lower is evaluated first; allow wins over block at the same priority. */
  priority: number;
  hitCount: number;
  createdAt: string;
  updatedAt: string;
};

/** Mirrors `ipc::dto::RuleDraftDto` — what the form submits. Rust validates it. */
export type RuleDraft = {
  name: string;
  action: RuleAction;
  direction: RuleDirection;
  protocol: RuleProtocol;
  remoteIp: string | null;
  remotePort: number | null;
  localPort: number | null;
  processPath: string | null;
  userSid: string | null;
  enabled: boolean;
  priority: number;
};

/** Mirrors `ipc::rules::RuleScope` — how much of a flow a derived rule names. */
export type RuleScope = "endpoint" | "host" | "port" | "process";

/** Mirrors `ipc::dto::EnforcementStatusDto`. */
export type EnforcementStatus = {
  /** True once a WinDivert handle has opened, i.e. the driver is usable. */
  available: boolean;
  /** True while packets are being intercepted — only when a block rule is enabled. */
  enforcing: boolean;
  /** True while socket events are being observed for attribution. */
  observing: boolean;
  enabledRules: number;
  blockRules: number;
  blockedPackets: number;
  socketEvents: number;
  trackedEndpoints: number;
  error: string | null;
};

/** Mirrors `ipc::dto::FlowHistorySummaryDto`. */
export type FlowHistorySummary = {
  storedFlows: number;
  oldestSeen: string | null;
  newestSeen: string | null;
  retentionHours: number;
  maxRows: number;
};

/** Mirrors `ipc::dto::PruneResultDto`. */
export type PruneResult = {
  removedCount: number;
  summary: FlowHistorySummary;
};

/** Mirrors `ipc::dto::SystemHealthDto`. */
export type SystemHealth = {
  isElevated: boolean;
  databaseReady: boolean;
  databasePath: string;
  logDirectory: string;
  npcapAvailable: boolean;
  activeInterfacesCount: number;
  activeFlowsCount: number;
  activeRulesCount: number;
  enforcementStatus: EnforcementStatus;
};

/** Mirrors `ipc::dto::CaptureInterfaceDto`. */
export type CaptureInterface = {
  id: string;
  name: string;
  addresses: string[];
  isLoopback: boolean;
  isUp: boolean;
  isWireless: boolean;
  isRecommended: boolean;
};

export type CaptureStatus = { running: boolean; interfaces: CaptureInterface[]; error: string | null };

/** Mirrors `ipc::dto::StreamStatsDto`. */
export type StreamStats = {
  packetsCaptured: number;
  bytesCaptured: number;
  packetsDropped: number;
  packetsPerSecond: number;
  bytesPerSecond: number;
  activeFlows: number;
};

/** Mirrors `ipc::dto::StreamMessage`. */
export type StreamMessage =
  | { type: "packets"; data: Packet[] }
  /** Only flows that changed since the last flush — patch, do not replace. */
  | { type: "flows"; data: Flow[] }
  /** Ids of flows that aged out of the live table. */
  | { type: "flowsRetired"; data: number[] }
  | { type: "stats"; data: StreamStats }
  | { type: "captureError"; data: { message: string } };
