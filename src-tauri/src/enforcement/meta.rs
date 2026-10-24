//! Reduces an intercepted IP packet to the facts a rule is matched against.
//!
//! WinDivert hands over raw IP packets, not Ethernet frames, and direction comes from the
//! driver's own outbound flag rather than the address heuristics the capture path needs.
//! Nothing here touches WinDivert types, so it is testable from byte fixtures.

use std::net::IpAddr;

use etherparse::{NetSlice, SlicedPacket, TransportSlice};

use crate::model::{Direction, FlowKey, TransportProtocol};

/// One intercepted packet, split into local and remote by direction so it lines up with
/// [`FlowKey`] and with how rules are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketMeta {
    pub direction: Direction,
    pub protocol: TransportProtocol,
    pub local_ip: IpAddr,
    /// Zero for the port-less protocols and for non-initial IP fragments.
    pub local_port: u16,
    pub remote_ip: IpAddr,
    pub remote_port: u16,
    pub length: usize,
}

impl PacketMeta {
    /// `None` when the bytes are not a packet we model: a truncated header, or a
    /// transport this build has no rule vocabulary for. Undecodable packets are always
    /// reinjected, so returning `None` fails open.
    pub fn decode(packet: &[u8], outbound: bool) -> Option<Self> {
        let sliced = SlicedPacket::from_ip(packet).ok()?;
        let (source_ip, dest_ip) = match sliced.net.as_ref()? {
            NetSlice::Ipv4(ipv4) => {
                let header = ipv4.header();
                (
                    IpAddr::V4(header.source_addr()),
                    IpAddr::V4(header.destination_addr()),
                )
            }
            NetSlice::Ipv6(ipv6) => {
                let header = ipv6.header();
                (
                    IpAddr::V6(header.source_addr()),
                    IpAddr::V6(header.destination_addr()),
                )
            }
            // WinDivert's network layer only carries IP, so ARP cannot appear here.
            NetSlice::Arp(_) => return None,
        };

        // A non-initial fragment has no transport header. It still belongs to a flow, so
        // it is matched with zero ports rather than dropped from consideration.
        let (protocol, source_port, dest_port) = match sliced.transport.as_ref() {
            Some(TransportSlice::Tcp(tcp)) => (
                TransportProtocol::Tcp,
                tcp.source_port(),
                tcp.destination_port(),
            ),
            Some(TransportSlice::Udp(udp)) => (
                TransportProtocol::Udp,
                udp.source_port(),
                udp.destination_port(),
            ),
            Some(TransportSlice::Icmpv4(_) | TransportSlice::Icmpv6(_)) => {
                (TransportProtocol::Icmp, 0, 0)
            }
            Some(TransportSlice::Igmp(_)) => (TransportProtocol::Igmp, 0, 0),
            None => (transport_from_header(&sliced)?, 0, 0),
        };

        let direction = if outbound {
            Direction::Out
        } else {
            Direction::In
        };
        let (local_ip, local_port, remote_ip, remote_port) = if outbound {
            (source_ip, source_port, dest_ip, dest_port)
        } else {
            (dest_ip, dest_port, source_ip, source_port)
        };

        Some(Self {
            direction,
            protocol,
            local_ip,
            local_port,
            remote_ip,
            remote_port,
            length: packet.len(),
        })
    }

    /// The flow this packet belongs to, so a drop can be marked on the row the user is
    /// looking at in the Connections view.
    pub fn flow_key(&self) -> FlowKey {
        FlowKey {
            protocol: self.protocol,
            local_ip: self.local_ip,
            local_port: self.local_port,
            remote_ip: self.remote_ip,
            remote_port: self.remote_port,
        }
    }
}

/// Transport of a packet whose header etherparse could not slice — a fragment, or an
/// IPv6 packet ending in an extension header. Read from the IP header's protocol number.
fn transport_from_header(sliced: &SlicedPacket<'_>) -> Option<TransportProtocol> {
    let protocol = match sliced.net.as_ref()? {
        NetSlice::Ipv4(ipv4) => ipv4.header().protocol(),
        NetSlice::Ipv6(ipv6) => ipv6.header().next_header(),
        NetSlice::Arp(_) => return None,
    };
    match protocol {
        etherparse::IpNumber::TCP => Some(TransportProtocol::Tcp),
        etherparse::IpNumber::UDP => Some(TransportProtocol::Udp),
        etherparse::IpNumber::ICMP | etherparse::IpNumber::IPV6_ICMP => {
            Some(TransportProtocol::Icmp)
        }
        etherparse::IpNumber::IGMP => Some(TransportProtocol::Igmp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use etherparse::PacketBuilder;

    fn tcp_packet() -> Vec<u8> {
        let builder = PacketBuilder::ipv4([192, 168, 1, 20], [93, 184, 216, 34], 64)
            .tcp(51_000, 443, 1, 64_240);
        let mut bytes = Vec::with_capacity(builder.size(4));
        builder
            .write(&mut bytes, &[1, 2, 3, 4])
            .expect("fixture should serialize");
        bytes
    }

    fn udp_v6_packet() -> Vec<u8> {
        let builder = PacketBuilder::ipv6(
            [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            [
                0x26, 0x06, 0x47, 0, 0x47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x11, 0x11,
            ],
            64,
        )
        .udp(50_000, 53);
        let mut bytes = Vec::with_capacity(builder.size(2));
        builder
            .write(&mut bytes, &[0, 1])
            .expect("fixture should serialize");
        bytes
    }

    #[test]
    fn outbound_packets_take_the_source_as_local() {
        let meta = PacketMeta::decode(&tcp_packet(), true).expect("should decode");
        assert_eq!(meta.direction, Direction::Out);
        assert_eq!(meta.protocol, TransportProtocol::Tcp);
        assert_eq!(meta.local_ip, "192.168.1.20".parse::<IpAddr>().unwrap());
        assert_eq!(meta.local_port, 51_000);
        assert_eq!(meta.remote_ip, "93.184.216.34".parse::<IpAddr>().unwrap());
        assert_eq!(meta.remote_port, 443);
    }

    #[test]
    fn inbound_packets_take_the_destination_as_local() {
        let meta = PacketMeta::decode(&tcp_packet(), false).expect("should decode");
        assert_eq!(meta.direction, Direction::In);
        assert_eq!(meta.local_ip, "93.184.216.34".parse::<IpAddr>().unwrap());
        assert_eq!(meta.local_port, 443);
        assert_eq!(meta.remote_port, 51_000);
    }

    #[test]
    fn decodes_ipv6_udp() {
        let meta = PacketMeta::decode(&udp_v6_packet(), true).expect("should decode");
        assert_eq!(meta.protocol, TransportProtocol::Udp);
        assert_eq!(meta.remote_port, 53);
        assert!(meta.local_ip.is_ipv6());
    }

    #[test]
    fn the_flow_key_matches_the_capture_path_shape() {
        let meta = PacketMeta::decode(&tcp_packet(), true).expect("should decode");
        let key = meta.flow_key();
        assert_eq!(key.protocol, TransportProtocol::Tcp);
        assert_eq!(key.local_port, 51_000);
        assert_eq!(key.remote_port, 443);
    }

    #[test]
    fn truncation_never_panics() {
        let packet = tcp_packet();
        for length in 0..packet.len() {
            let _ = PacketMeta::decode(&packet[..length], true);
        }
    }

    #[test]
    fn rejects_bytes_that_are_not_ip() {
        assert!(PacketMeta::decode(&[0xff; 20], true).is_none());
        assert!(PacketMeta::decode(&[], true).is_none());
    }
}
