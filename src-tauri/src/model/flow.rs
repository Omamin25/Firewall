use std::net::IpAddr;

use chrono::{DateTime, Utc};

use super::{Attribution, Direction, L7Protocol, MatchedRule, Packet};

/// The transport a flow rides on. Flows exist only for IP conversations, so ARP and
/// other link-only frames never produce one.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransportProtocol {
    Tcp,
    Udp,
    Icmp,
    Igmp,
}

impl TransportProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            TransportProtocol::Tcp => "tcp",
            TransportProtocol::Udp => "udp",
            TransportProtocol::Icmp => "icmp",
            TransportProtocol::Igmp => "igmp",
        }
    }

    /// True for the protocols that carry ports, i.e. the ones the OS connection tables
    /// can attribute to a process.
    pub fn has_ports(self) -> bool {
        matches!(self, TransportProtocol::Tcp | TransportProtocol::Udp)
    }
}

/// Identity of a connection. Local/remote are already split by direction on the packet,
/// so both halves of a conversation normalise to the same key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FlowKey {
    pub protocol: TransportProtocol,
    pub local_ip: IpAddr,
    /// Zero for the port-less protocols (ICMP/IGMP).
    pub local_port: u16,
    pub remote_ip: IpAddr,
    pub remote_port: u16,
}

impl FlowKey {
    /// `None` when the frame has no IP conversation to aggregate (ARP, malformed L3,
    /// or a transport this build does not model).
    pub fn from_packet(packet: &Packet) -> Option<Self> {
        use crate::model::layers::{L3Detail, L4Detail};

        // ARP carries IPv4 addresses but no transport, so it is deliberately excluded.
        if matches!(packet.layers.l3.as_ref()?, L3Detail::Arp { .. }) {
            return None;
        }
        let protocol = match packet.layers.l4.as_ref()? {
            L4Detail::Tcp { .. } => TransportProtocol::Tcp,
            L4Detail::Udp { .. } => TransportProtocol::Udp,
            L4Detail::Icmp { .. } => TransportProtocol::Icmp,
            L4Detail::Igmp { .. } => TransportProtocol::Igmp,
        };
        Some(Self {
            protocol,
            local_ip: packet.local_addr()?,
            local_port: packet.local_port().unwrap_or(0),
            remote_ip: packet.remote_addr()?,
            remote_port: packet.remote_port().unwrap_or(0),
        })
    }
}

/// An aggregated connection: one row of the Connections view and one row of the
/// `flows` history table. Snapshots are cloned out of the flow table for IPC.
#[derive(Clone, Debug)]
pub struct Flow {
    pub id: u64,
    pub key: FlowKey,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    /// Direction of the first frame observed — which side opened the conversation.
    pub direction: Direction,
    /// Most specific label seen so far; an application protocol never regresses to
    /// its transport once classified.
    pub protocol: L7Protocol,
    pub remote_host: Option<String>,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets: u64,
    /// Set by a TCP FIN/RST. Closed flows stay visible until they age out.
    pub closed: bool,
    pub attribution: Attribution,
    /// Set once the enforcement engine dropped a packet of this conversation. It stays
    /// set for the flow's lifetime: the connection was interfered with, so the row keeps
    /// saying so even after the rule is toggled off.
    pub blocked: bool,
    /// The rule that most recently dropped a packet of this flow.
    pub matched_rule: Option<MatchedRule>,
}

impl Flow {
    pub fn new(id: u64, key: FlowKey, packet: &Packet) -> Self {
        let mut flow = Self {
            id,
            key,
            first_seen: packet.timestamp,
            last_seen: packet.timestamp,
            direction: packet.direction,
            protocol: packet.protocol,
            remote_host: None,
            bytes_sent: 0,
            bytes_recv: 0,
            packets: 0,
            closed: false,
            attribution: Attribution::default(),
            blocked: false,
            matched_rule: None,
        };
        flow.record(packet);
        flow
    }

    /// Folds one frame into the counters. Cheap enough to run on the decode thread.
    pub fn record(&mut self, packet: &Packet) {
        let bytes = packet.frame_len as u64;
        match packet.direction {
            Direction::Out => self.bytes_sent += bytes,
            Direction::In => self.bytes_recv += bytes,
        }
        self.packets += 1;
        // Capture threads interleave, so timestamps can arrive slightly out of order.
        self.last_seen = self.last_seen.max(packet.timestamp);
        self.first_seen = self.first_seen.min(packet.timestamp);
        if packet.protocol.is_application() && !self.protocol.is_application() {
            self.protocol = packet.protocol;
        }
        if self.remote_host.is_none() {
            self.remote_host.clone_from(&packet.remote_host);
        }
        if packet.closes_connection() {
            self.closed = true;
        }
    }

    pub fn bytes_total(&self) -> u64 {
        self.bytes_sent + self.bytes_recv
    }

    /// Wall-clock span of the conversation. Never negative: `record` keeps the bounds
    /// ordered even when frames arrive out of sequence.
    pub fn duration_ms(&self) -> u64 {
        (self.last_seen - self.first_seen).num_milliseconds().max(0) as u64
    }

    /// Newest activity, used for idle expiry and eviction ordering.
    pub fn idle_for(&self, now: DateTime<Utc>) -> chrono::Duration {
        now - self.last_seen
    }
}
