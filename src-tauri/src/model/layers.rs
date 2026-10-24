use std::net::{Ipv4Addr, Ipv6Addr};

/// Layer-by-layer decode of a single captured frame. Every layer is optional
/// below L2 because malformed or non-IP frames must still be representable.
#[derive(Clone, Debug)]
pub struct PacketLayer {
    pub l2: L2Detail,
    pub l3: Option<L3Detail>,
    pub l4: Option<L4Detail>,
    pub l7: Option<L7Detail>,
    pub payload_len: usize,
}

#[derive(Clone, Debug)]
pub struct L2Detail {
    pub source_mac: [u8; 6],
    pub dest_mac: [u8; 6],
    pub ether_type: u16,
}

#[derive(Clone, Debug)]
pub enum L3Detail {
    Ipv4 {
        source: Ipv4Addr,
        dest: Ipv4Addr,
        ttl: u8,
        protocol: u8,
        dont_fragment: bool,
        more_fragments: bool,
        fragment_offset: u16,
    },
    Ipv6 {
        source: Ipv6Addr,
        dest: Ipv6Addr,
        hop_limit: u8,
        next_header: u8,
    },
    Arp {
        operation: u16,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    },
}

#[derive(Clone, Debug)]
pub enum L4Detail {
    Tcp {
        source_port: u16,
        dest_port: u16,
        sequence: u32,
        acknowledgment: u32,
        flags: TcpFlags,
        window: u16,
    },
    Udp {
        source_port: u16,
        dest_port: u16,
        length: u16,
    },
    Icmp {
        kind: u8,
        code: u8,
    },
    /// IGMP rides directly on IP like ICMP, so it lives at the same level here.
    Igmp {
        kind: u8,
    },
}

impl L4Detail {
    /// Source/destination ports, or `None` for the port-less protocols.
    pub fn ports(&self) -> Option<(u16, u16)> {
        match self {
            L4Detail::Tcp {
                source_port,
                dest_port,
                ..
            }
            | L4Detail::Udp {
                source_port,
                dest_port,
                ..
            } => Some((*source_port, *dest_port)),
            L4Detail::Icmp { .. } | L4Detail::Igmp { .. } => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TcpFlags {
    pub fin: bool,
    pub syn: bool,
    pub rst: bool,
    pub psh: bool,
    pub ack: bool,
    pub urg: bool,
    pub ece: bool,
    pub cwr: bool,
}

impl TcpFlags {
    /// Compact wire-style rendering (`[SYN, ACK]`) for the inspector.
    pub fn labels(&self) -> Vec<&'static str> {
        let mut set = Vec::new();
        if self.syn {
            set.push("SYN");
        }
        if self.ack {
            set.push("ACK");
        }
        if self.fin {
            set.push("FIN");
        }
        if self.rst {
            set.push("RST");
        }
        if self.psh {
            set.push("PSH");
        }
        if self.urg {
            set.push("URG");
        }
        if self.ece {
            set.push("ECE");
        }
        if self.cwr {
            set.push("CWR");
        }
        set
    }
}

/// Application-layer detail extracted by the `parse::l7` classifiers.
/// `Opaque` carries a hint (usually "port 443" style) so the UI never shows a blank layer.
#[derive(Clone, Debug)]
pub enum L7Detail {
    Dns {
        is_response: bool,
        questions: Vec<String>,
        answers: Vec<String>,
    },
    Http {
        method: Option<String>,
        host: Option<String>,
        path: Option<String>,
        status: Option<u16>,
    },
    Tls {
        sni: Option<String>,
        version: Option<String>,
        handshake: &'static str,
    },
    Quic {
        sni: Option<String>,
        long_header: bool,
    },
    Dhcp {
        message_type: &'static str,
    },
    Mdns {
        is_response: bool,
        questions: Vec<String>,
    },
    Opaque {
        hint: String,
    },
}
