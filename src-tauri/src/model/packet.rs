use std::net::IpAddr;

use chrono::{DateTime, Utc};

use super::layers::{L3Detail, L4Detail, L7Detail, PacketLayer};

/// A fully decoded captured frame. Attribution is not stored here: it belongs to the
/// owning flow, which keeps improving after the frame was decoded, so both the table row
/// and the inspector read it from there via `flow_id`.
#[derive(Clone, Debug)]
pub struct Packet {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub interface: String,
    pub direction: Direction,
    pub protocol: L7Protocol,
    /// Best-known remote name (DNS answer, TLS SNI); falls back to the IP in the UI.
    pub remote_host: Option<String>,
    pub frame_len: usize,
    pub captured_len: usize,
    pub layers: PacketLayer,
    /// Set once the frame is folded into a flow; `None` for frames with no IP
    /// conversation (ARP) so the inspector knows not to look one up.
    pub flow_id: Option<u64>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
        }
    }
}

/// The protocol label shown in the traffic table. Derived from the L7 classifier
/// when it recognises the payload, otherwise from the L4 header.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum L7Protocol {
    Dns,
    Http,
    Tls,
    Quic,
    Dhcp,
    Mdns,
    Tcp,
    Udp,
    Icmp,
    Igmp,
    Arp,
    Other,
}

impl L7Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            L7Protocol::Dns => "DNS",
            L7Protocol::Http => "HTTP",
            L7Protocol::Tls => "TLS",
            L7Protocol::Quic => "QUIC",
            L7Protocol::Dhcp => "DHCP",
            L7Protocol::Mdns => "mDNS",
            L7Protocol::Tcp => "TCP",
            L7Protocol::Udp => "UDP",
            L7Protocol::Icmp => "ICMP",
            L7Protocol::Igmp => "IGMP",
            L7Protocol::Arp => "ARP",
            L7Protocol::Other => "Other",
        }
    }

    /// Token key for `--proto-*` colors in `ui_tokens.md`.
    pub fn token(self) -> &'static str {
        match self {
            L7Protocol::Dns | L7Protocol::Mdns => "dns",
            L7Protocol::Http => "http",
            L7Protocol::Tls => "tls",
            L7Protocol::Quic => "quic",
            L7Protocol::Icmp => "icmp",
            _ => "other",
        }
    }

    /// True when a classifier recognised the payload rather than falling back to the
    /// transport header. A flow keeps the most specific label it has ever seen, so a
    /// TLS conversation is not relabelled "TCP" by its next bare ACK.
    pub fn is_application(self) -> bool {
        matches!(
            self,
            L7Protocol::Dns
                | L7Protocol::Http
                | L7Protocol::Tls
                | L7Protocol::Quic
                | L7Protocol::Dhcp
                | L7Protocol::Mdns
        )
    }
}

impl Packet {
    /// Local/remote split follows `direction`: for outbound frames the source is
    /// local, for inbound the destination is.
    pub fn local_addr(&self) -> Option<IpAddr> {
        self.addr_pair().map(|(source, dest)| match self.direction {
            Direction::Out => source,
            Direction::In => dest,
        })
    }

    pub fn remote_addr(&self) -> Option<IpAddr> {
        self.addr_pair().map(|(source, dest)| match self.direction {
            Direction::Out => dest,
            Direction::In => source,
        })
    }

    pub fn local_port(&self) -> Option<u16> {
        self.port_pair().map(|(source, dest)| match self.direction {
            Direction::Out => source,
            Direction::In => dest,
        })
    }

    pub fn remote_port(&self) -> Option<u16> {
        self.port_pair().map(|(source, dest)| match self.direction {
            Direction::Out => dest,
            Direction::In => source,
        })
    }

    fn addr_pair(&self) -> Option<(IpAddr, IpAddr)> {
        match self.layers.l3.as_ref()? {
            L3Detail::Ipv4 { source, dest, .. } => Some((IpAddr::V4(*source), IpAddr::V4(*dest))),
            L3Detail::Ipv6 { source, dest, .. } => Some((IpAddr::V6(*source), IpAddr::V6(*dest))),
            L3Detail::Arp {
                sender_ip,
                target_ip,
                ..
            } => Some((IpAddr::V4(*sender_ip), IpAddr::V4(*target_ip))),
        }
    }

    fn port_pair(&self) -> Option<(u16, u16)> {
        self.layers.l4.as_ref().and_then(L4Detail::ports)
    }

    /// True for a TCP FIN or RST — the signal that the owning flow is finished. UDP has
    /// no teardown, so those flows only ever close by ageing out.
    pub fn closes_connection(&self) -> bool {
        matches!(
            self.layers.l4.as_ref(),
            Some(L4Detail::Tcp { flags, .. }) if flags.fin || flags.rst
        )
    }

    /// Short summary line for the traffic table's detail column.
    pub fn summary(&self) -> String {
        match self.layers.l7.as_ref() {
            Some(L7Detail::Dns {
                questions,
                is_response,
                ..
            }) => {
                let name = questions.first().map(String::as_str).unwrap_or("query");
                if *is_response {
                    format!("response {name}")
                } else {
                    format!("query {name}")
                }
            }
            Some(L7Detail::Http {
                method,
                host,
                path,
                status,
            }) => match (method, host, path, status) {
                (Some(method), Some(host), Some(path), _) => format!("{method} {host}{path}"),
                (_, _, _, Some(status)) => format!("{status} response"),
                _ => "HTTP".to_owned(),
            },
            Some(L7Detail::Tls { sni, handshake, .. }) => match sni {
                Some(sni) => format!("{handshake} {sni}"),
                None => (*handshake).to_owned(),
            },
            Some(L7Detail::Quic { sni, .. }) => match sni {
                Some(sni) => format!("Initial {sni}"),
                None => "QUIC".to_owned(),
            },
            Some(L7Detail::Dhcp { message_type }) => (*message_type).to_owned(),
            Some(L7Detail::Mdns { questions, .. }) => questions
                .first()
                .cloned()
                .unwrap_or_else(|| "mDNS".to_owned()),
            Some(L7Detail::Opaque { hint }) => hint.clone(),
            None => self.protocol.as_str().to_owned(),
        }
    }
}
