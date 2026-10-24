//! Byte-fixture builders for the parser tests.
//!
//! Frames are assembled by hand so each test states exactly which bytes it exercises;
//! this keeps the fixtures readable and avoids committing opaque `.pcap` blobs.

// Each test binary includes the whole module but uses only the builders it needs.
#![allow(dead_code)]

pub mod l7;

pub const LOCAL_MAC: [u8; 6] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
pub const GATEWAY_MAC: [u8; 6] = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
pub const LOCAL_IPV4: [u8; 4] = [192, 168, 1, 20];
pub const REMOTE_IPV4: [u8; 4] = [93, 184, 216, 34];

pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_IPV6: u16 = 0x86dd;
pub const ETHERTYPE_ARP: u16 = 0x0806;

pub const IP_PROTO_ICMP: u8 = 1;
pub const IP_PROTO_TCP: u8 = 6;
pub const IP_PROTO_UDP: u8 = 17;

/// Ethernet II header: destination, source, EtherType.
pub fn ethernet(dest: [u8; 6], source: [u8; 6], ether_type: u16) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14);
    frame.extend_from_slice(&dest);
    frame.extend_from_slice(&source);
    frame.extend_from_slice(&ether_type.to_be_bytes());
    frame
}

/// IPv4 header with a correct total-length field and a zeroed checksum.
/// `etherparse` does not verify the checksum when slicing, so tests stay readable.
pub fn ipv4(source: [u8; 4], dest: [u8; 4], protocol: u8, payload_len: usize, ttl: u8) -> Vec<u8> {
    let total_len = (20 + payload_len) as u16;
    let mut header = Vec::with_capacity(20);
    header.push(0x45); // version 4, IHL 5
    header.push(0x00); // DSCP/ECN
    header.extend_from_slice(&total_len.to_be_bytes());
    header.extend_from_slice(&[0x00, 0x01]); // identification
    header.extend_from_slice(&[0x40, 0x00]); // flags: don't fragment
    header.push(ttl);
    header.push(protocol);
    header.extend_from_slice(&[0x00, 0x00]); // checksum (unchecked)
    header.extend_from_slice(&source);
    header.extend_from_slice(&dest);
    header
}

/// IPv6 header (fixed 40 bytes).
pub fn ipv6(
    source: [u8; 16],
    dest: [u8; 16],
    next_header: u8,
    payload_len: usize,
    hop_limit: u8,
) -> Vec<u8> {
    let mut header = Vec::with_capacity(40);
    header.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]); // version 6, no traffic class/flow label
    header.extend_from_slice(&(payload_len as u16).to_be_bytes());
    header.push(next_header);
    header.push(hop_limit);
    header.extend_from_slice(&source);
    header.extend_from_slice(&dest);
    header
}

/// Minimal 20-byte TCP header. `flags` is the raw flag byte (bit 1 = SYN, bit 4 = ACK).
pub fn tcp(
    source_port: u16,
    dest_port: u16,
    sequence: u32,
    ack: u32,
    flags: u8,
    window: u16,
) -> Vec<u8> {
    let mut header = Vec::with_capacity(20);
    header.extend_from_slice(&source_port.to_be_bytes());
    header.extend_from_slice(&dest_port.to_be_bytes());
    header.extend_from_slice(&sequence.to_be_bytes());
    header.extend_from_slice(&ack.to_be_bytes());
    header.push(0x50); // data offset 5 words, no flags in the reserved nibble
    header.push(flags);
    header.extend_from_slice(&window.to_be_bytes());
    header.extend_from_slice(&[0x00, 0x00]); // checksum (unchecked)
    header.extend_from_slice(&[0x00, 0x00]); // urgent pointer
    header
}

pub const TCP_FIN: u8 = 0x01;
pub const TCP_SYN: u8 = 0x02;
pub const TCP_RST: u8 = 0x04;
pub const TCP_PSH: u8 = 0x08;
pub const TCP_ACK: u8 = 0x10;

/// UDP header; `length` covers the header plus payload.
pub fn udp(source_port: u16, dest_port: u16, payload_len: usize) -> Vec<u8> {
    let mut header = Vec::with_capacity(8);
    header.extend_from_slice(&source_port.to_be_bytes());
    header.extend_from_slice(&dest_port.to_be_bytes());
    header.extend_from_slice(&((8 + payload_len) as u16).to_be_bytes());
    header.extend_from_slice(&[0x00, 0x00]); // checksum (unchecked)
    header
}

/// ICMPv4 echo request/reply header.
pub fn icmpv4(kind: u8, code: u8) -> Vec<u8> {
    vec![kind, code, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01]
}

/// Ethernet/IPv4 ARP packet.
pub fn arp(
    operation: u16,
    sender_mac: [u8; 6],
    sender_ip: [u8; 4],
    target_mac: [u8; 6],
    target_ip: [u8; 4],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(28);
    packet.extend_from_slice(&1_u16.to_be_bytes()); // hardware type: Ethernet
    packet.extend_from_slice(&ETHERTYPE_IPV4.to_be_bytes()); // protocol type: IPv4
    packet.push(6); // hardware address length
    packet.push(4); // protocol address length
    packet.extend_from_slice(&operation.to_be_bytes());
    packet.extend_from_slice(&sender_mac);
    packet.extend_from_slice(&sender_ip);
    packet.extend_from_slice(&target_mac);
    packet.extend_from_slice(&target_ip);
    packet
}

/// Assembles an outbound IPv4/TCP frame carrying `payload`.
pub fn ipv4_tcp_frame(source_port: u16, dest_port: u16, flags: u8, payload: &[u8]) -> Vec<u8> {
    let transport = tcp(source_port, dest_port, 0x1000, 0x2000, flags, 64_240);
    let mut frame = ethernet(GATEWAY_MAC, LOCAL_MAC, ETHERTYPE_IPV4);
    frame.extend_from_slice(&ipv4(
        LOCAL_IPV4,
        REMOTE_IPV4,
        IP_PROTO_TCP,
        transport.len() + payload.len(),
        128,
    ));
    frame.extend_from_slice(&transport);
    frame.extend_from_slice(payload);
    frame
}

/// Assembles the inbound reply to [`ipv4_tcp_frame`] — the same conversation with the
/// endpoints swapped, so flow aggregation can be exercised in both directions.
pub fn ipv4_tcp_reply_frame(
    source_port: u16,
    dest_port: u16,
    flags: u8,
    payload: &[u8],
) -> Vec<u8> {
    let transport = tcp(source_port, dest_port, 0x2000, 0x1001, flags, 64_240);
    let mut frame = ethernet(LOCAL_MAC, GATEWAY_MAC, ETHERTYPE_IPV4);
    frame.extend_from_slice(&ipv4(
        REMOTE_IPV4,
        LOCAL_IPV4,
        IP_PROTO_TCP,
        transport.len() + payload.len(),
        54,
    ));
    frame.extend_from_slice(&transport);
    frame.extend_from_slice(payload);
    frame
}

/// Assembles an outbound IPv4/UDP frame carrying `payload`.
pub fn ipv4_udp_frame(source_port: u16, dest_port: u16, payload: &[u8]) -> Vec<u8> {
    let transport = udp(source_port, dest_port, payload.len());
    let mut frame = ethernet(GATEWAY_MAC, LOCAL_MAC, ETHERTYPE_IPV4);
    frame.extend_from_slice(&ipv4(
        LOCAL_IPV4,
        REMOTE_IPV4,
        IP_PROTO_UDP,
        transport.len() + payload.len(),
        128,
    ));
    frame.extend_from_slice(&transport);
    frame.extend_from_slice(payload);
    frame
}
