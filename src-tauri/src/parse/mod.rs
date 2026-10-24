pub mod l7;

use std::net::Ipv4Addr;

use etherparse::{LinkSlice, NetSlice, SlicedPacket, TransportSlice};

use crate::model::{
    layers::{L2Detail, L3Detail, L4Detail, TcpFlags},
    Direction, L7Protocol, PacketLayer,
};

/// Everything the decoder can learn from a frame in isolation. The capture layer
/// supplies interface identity and timestamps; direction needs the interface's own
/// addresses, so it is passed in via [`DecodeContext`].
#[derive(Clone, Debug)]
pub struct Decoded {
    pub layers: PacketLayer,
    pub direction: Direction,
    pub protocol: L7Protocol,
    pub remote_host: Option<String>,
}

/// Per-interface facts needed to classify direction. Held by the capture thread and
/// reused for every frame on that interface.
#[derive(Clone, Debug, Default)]
pub struct DecodeContext {
    /// MAC addresses belonging to this host's adapter, when known.
    pub local_macs: Vec<[u8; 6]>,
    /// IP addresses assigned to this interface.
    pub local_ips: Vec<std::net::IpAddr>,
}

/// Decodes an Ethernet frame across L2–L7. Returns `None` only when the frame has no
/// usable link header — every other malformation degrades to a partially-filled
/// [`PacketLayer`] rather than failing, so the UI still shows what was observed.
pub fn decode(frame: &[u8], context: &DecodeContext) -> Option<Decoded> {
    let sliced = SlicedPacket::from_ethernet(frame).ok()?;
    let l2 = link_detail(&sliced)?;
    let l3 = sliced.net.as_ref().map(net_detail);
    let l4 = sliced.transport.as_ref().map(transport_detail);
    let payload = transport_payload(&sliced);

    let direction = infer_direction(&l2, l3.as_ref(), context);
    let l7 = l4
        .as_ref()
        .and_then(|l4| l7::classify(l4, payload, l3.as_ref()));
    let protocol = protocol_label(l3.as_ref(), l4.as_ref(), l7.as_ref());
    let remote_host = l7.as_ref().and_then(l7::remote_host);

    Some(Decoded {
        layers: PacketLayer {
            l2,
            l3,
            l4,
            l7,
            payload_len: payload.len(),
        },
        direction,
        protocol,
        remote_host,
    })
}

fn link_detail(sliced: &SlicedPacket<'_>) -> Option<L2Detail> {
    match sliced.link.as_ref()? {
        LinkSlice::Ethernet2(ethernet) => Some(L2Detail {
            source_mac: ethernet.source(),
            dest_mac: ethernet.destination(),
            ether_type: ethernet.ether_type().0,
        }),
        // Npcap always delivers Ethernet II on the adapters we open; the remaining
        // link variants (SLL, bare payloads) are not reachable on Windows.
        _ => None,
    }
}

fn net_detail(net: &NetSlice<'_>) -> L3Detail {
    match net {
        NetSlice::Ipv4(ipv4) => {
            let header = ipv4.header();
            L3Detail::Ipv4 {
                source: header.source_addr(),
                dest: header.destination_addr(),
                ttl: header.ttl(),
                protocol: header.protocol().0,
                dont_fragment: header.dont_fragment(),
                more_fragments: header.more_fragments(),
                fragment_offset: header.fragments_offset().value(),
            }
        }
        NetSlice::Ipv6(ipv6) => {
            let header = ipv6.header();
            L3Detail::Ipv6 {
                source: header.source_addr(),
                dest: header.destination_addr(),
                hop_limit: header.hop_limit(),
                next_header: header.next_header().0,
            }
        }
        NetSlice::Arp(arp) => L3Detail::Arp {
            operation: arp.operation().0,
            sender_ip: ipv4_from_slice(arp.sender_protocol_addr()),
            target_ip: ipv4_from_slice(arp.target_protocol_addr()),
        },
    }
}

fn ipv4_from_slice(bytes: &[u8]) -> Ipv4Addr {
    match bytes {
        [a, b, c, d] => Ipv4Addr::new(*a, *b, *c, *d),
        // Non-IPv4 ARP (e.g. ATM) is vanishingly rare on a host adapter; show it as
        // unspecified rather than dropping the frame.
        _ => Ipv4Addr::UNSPECIFIED,
    }
}

fn transport_detail(transport: &TransportSlice<'_>) -> L4Detail {
    match transport {
        TransportSlice::Tcp(tcp) => L4Detail::Tcp {
            source_port: tcp.source_port(),
            dest_port: tcp.destination_port(),
            sequence: tcp.sequence_number(),
            acknowledgment: tcp.acknowledgment_number(),
            flags: TcpFlags {
                fin: tcp.fin(),
                syn: tcp.syn(),
                rst: tcp.rst(),
                psh: tcp.psh(),
                ack: tcp.ack(),
                urg: tcp.urg(),
                ece: tcp.ece(),
                cwr: tcp.cwr(),
            },
            window: tcp.window_size(),
        },
        TransportSlice::Udp(udp) => L4Detail::Udp {
            source_port: udp.source_port(),
            dest_port: udp.destination_port(),
            length: udp.length(),
        },
        TransportSlice::Icmpv4(icmp) => L4Detail::Icmp {
            kind: icmp.type_u8(),
            code: icmp.code_u8(),
        },
        TransportSlice::Icmpv6(icmp) => L4Detail::Icmp {
            kind: icmp.type_u8(),
            code: icmp.code_u8(),
        },
        TransportSlice::Igmp(igmp) => L4Detail::Igmp {
            kind: igmp.type_u8(),
        },
    }
}

fn transport_payload<'a>(sliced: &SlicedPacket<'a>) -> &'a [u8] {
    match sliced.transport.as_ref() {
        Some(TransportSlice::Tcp(tcp)) => tcp.payload(),
        Some(TransportSlice::Udp(udp)) => udp.payload(),
        Some(TransportSlice::Icmpv4(icmp)) => icmp.payload(),
        Some(TransportSlice::Icmpv6(icmp)) => icmp.payload(),
        Some(TransportSlice::Igmp(igmp)) => igmp.payload(),
        None => &[],
    }
}

/// Direction is decided by IP first (most reliable), then MAC, then a heuristic on
/// private-vs-public addressing. A wrong guess only mislabels the arrow in the UI —
/// enforcement in Phase 4 uses WinDivert's own direction flag.
fn infer_direction(l2: &L2Detail, l3: Option<&L3Detail>, context: &DecodeContext) -> Direction {
    if let Some(l3) = l3 {
        let (source, dest) = match l3 {
            L3Detail::Ipv4 { source, dest, .. } => {
                (std::net::IpAddr::V4(*source), std::net::IpAddr::V4(*dest))
            }
            L3Detail::Ipv6 { source, dest, .. } => {
                (std::net::IpAddr::V6(*source), std::net::IpAddr::V6(*dest))
            }
            L3Detail::Arp {
                sender_ip,
                target_ip,
                ..
            } => (
                std::net::IpAddr::V4(*sender_ip),
                std::net::IpAddr::V4(*target_ip),
            ),
        };
        let source_is_local = context.local_ips.contains(&source);
        let dest_is_local = context.local_ips.contains(&dest);
        if source_is_local != dest_is_local {
            return if source_is_local {
                Direction::Out
            } else {
                Direction::In
            };
        }
    }

    if !context.local_macs.is_empty() {
        let source_is_local = context.local_macs.contains(&l2.source_mac);
        let dest_is_local = context.local_macs.contains(&l2.dest_mac);
        if source_is_local != dest_is_local {
            return if source_is_local {
                Direction::Out
            } else {
                Direction::In
            };
        }
    }

    match l3 {
        Some(L3Detail::Ipv4 { source, dest, .. }) => {
            if is_private_v4(source) && !is_private_v4(dest) {
                Direction::Out
            } else {
                Direction::In
            }
        }
        _ => Direction::Out,
    }
}

fn is_private_v4(addr: &Ipv4Addr) -> bool {
    addr.is_private() || addr.is_link_local() || addr.is_loopback()
}

fn protocol_label(
    l3: Option<&L3Detail>,
    l4: Option<&L4Detail>,
    l7: Option<&crate::model::L7Detail>,
) -> L7Protocol {
    if let Some(protocol) = l7.and_then(l7::protocol_label) {
        return protocol;
    }
    match l4 {
        Some(L4Detail::Tcp { .. }) => L7Protocol::Tcp,
        Some(L4Detail::Udp { .. }) => L7Protocol::Udp,
        Some(L4Detail::Icmp { .. }) => L7Protocol::Icmp,
        Some(L4Detail::Igmp { .. }) => L7Protocol::Igmp,
        None => match l3 {
            Some(L3Detail::Arp { .. }) => L7Protocol::Arp,
            _ => L7Protocol::Other,
        },
    }
}
