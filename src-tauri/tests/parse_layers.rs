//! L2–L4 decode tests over hand-built Ethernet frames.

mod fixtures;

use cognton_lib::{
    model::{
        layers::{L3Detail, L4Detail},
        Direction, L7Protocol,
    },
    parse::{self, DecodeContext},
};
use fixtures::*;

fn local_context() -> DecodeContext {
    DecodeContext {
        local_macs: vec![LOCAL_MAC],
        local_ips: vec![std::net::IpAddr::V4(std::net::Ipv4Addr::new(
            192, 168, 1, 20,
        ))],
    }
}

#[test]
fn decodes_ethernet_header() {
    let frame = ipv4_tcp_frame(52_000, 443, TCP_SYN, &[]);
    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");

    assert_eq!(decoded.layers.l2.source_mac, LOCAL_MAC);
    assert_eq!(decoded.layers.l2.dest_mac, GATEWAY_MAC);
    assert_eq!(decoded.layers.l2.ether_type, ETHERTYPE_IPV4);
}

#[test]
fn decodes_ipv4_header_fields() {
    let frame = ipv4_tcp_frame(52_000, 443, TCP_SYN, &[]);
    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");

    match decoded.layers.l3.expect("IPv4 layer should be present") {
        L3Detail::Ipv4 {
            source,
            dest,
            ttl,
            protocol,
            dont_fragment,
            more_fragments,
            fragment_offset,
        } => {
            assert_eq!(source.octets(), LOCAL_IPV4);
            assert_eq!(dest.octets(), REMOTE_IPV4);
            assert_eq!(ttl, 128);
            assert_eq!(protocol, IP_PROTO_TCP);
            assert!(dont_fragment);
            assert!(!more_fragments);
            assert_eq!(fragment_offset, 0);
        }
        other => panic!("expected IPv4, got {other:?}"),
    }
}

#[test]
fn decodes_tcp_header_and_flags() {
    let frame = ipv4_tcp_frame(52_000, 443, TCP_SYN | TCP_ACK, &[]);
    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");

    match decoded.layers.l4.expect("TCP layer should be present") {
        L4Detail::Tcp {
            source_port,
            dest_port,
            sequence,
            acknowledgment,
            flags,
            window,
        } => {
            assert_eq!(source_port, 52_000);
            assert_eq!(dest_port, 443);
            assert_eq!(sequence, 0x1000);
            assert_eq!(acknowledgment, 0x2000);
            assert!(flags.syn && flags.ack);
            assert!(!flags.fin && !flags.rst && !flags.psh);
            assert_eq!(flags.labels(), vec!["SYN", "ACK"]);
            assert_eq!(window, 64_240);
        }
        other => panic!("expected TCP, got {other:?}"),
    }
}

#[test]
fn decodes_udp_header() {
    let payload = [0xde, 0xad, 0xbe, 0xef];
    let frame = ipv4_udp_frame(5_353, 5_353, &payload);
    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");

    match decoded.layers.l4.expect("UDP layer should be present") {
        L4Detail::Udp {
            source_port,
            dest_port,
            length,
        } => {
            assert_eq!(source_port, 5_353);
            assert_eq!(dest_port, 5_353);
            assert_eq!(length, (8 + payload.len()) as u16);
        }
        other => panic!("expected UDP, got {other:?}"),
    }
    assert_eq!(decoded.layers.payload_len, payload.len());
}

#[test]
fn decodes_ipv6_header() {
    let source = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let dest = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
    let transport = tcp(443, 52_000, 1, 2, TCP_ACK, 1_024);

    let mut frame = ethernet(LOCAL_MAC, GATEWAY_MAC, ETHERTYPE_IPV6);
    frame.extend_from_slice(&ipv6(source, dest, IP_PROTO_TCP, transport.len(), 64));
    frame.extend_from_slice(&transport);

    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");
    match decoded.layers.l3.expect("IPv6 layer should be present") {
        L3Detail::Ipv6 {
            source: decoded_source,
            hop_limit,
            next_header,
            ..
        } => {
            assert_eq!(decoded_source.octets(), source);
            assert_eq!(hop_limit, 64);
            assert_eq!(next_header, IP_PROTO_TCP);
        }
        other => panic!("expected IPv6, got {other:?}"),
    }
}

#[test]
fn decodes_arp_request() {
    let mut frame = ethernet([0xff; 6], LOCAL_MAC, ETHERTYPE_ARP);
    frame.extend_from_slice(&arp(1, LOCAL_MAC, LOCAL_IPV4, [0; 6], [192, 168, 1, 1]));

    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");
    match decoded.layers.l3.expect("ARP layer should be present") {
        L3Detail::Arp {
            operation,
            sender_ip,
            target_ip,
        } => {
            assert_eq!(operation, 1);
            assert_eq!(sender_ip.octets(), LOCAL_IPV4);
            assert_eq!(target_ip.octets(), [192, 168, 1, 1]);
        }
        other => panic!("expected ARP, got {other:?}"),
    }
    assert_eq!(decoded.protocol, L7Protocol::Arp);
}

#[test]
fn decodes_icmp_echo_request() {
    let transport = icmpv4(8, 0);
    let mut frame = ethernet(GATEWAY_MAC, LOCAL_MAC, ETHERTYPE_IPV4);
    frame.extend_from_slice(&ipv4(
        LOCAL_IPV4,
        REMOTE_IPV4,
        IP_PROTO_ICMP,
        transport.len(),
        64,
    ));
    frame.extend_from_slice(&transport);

    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");
    match decoded.layers.l4.expect("ICMP layer should be present") {
        L4Detail::Icmp { kind, code } => {
            assert_eq!(kind, 8);
            assert_eq!(code, 0);
        }
        other => panic!("expected ICMP, got {other:?}"),
    }
    assert_eq!(decoded.protocol, L7Protocol::Icmp);
}

#[test]
fn infers_outbound_direction_from_local_ip() {
    let frame = ipv4_tcp_frame(52_000, 443, TCP_SYN, &[]);
    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");
    assert_eq!(decoded.direction, Direction::Out);
}

#[test]
fn infers_inbound_direction_from_local_ip() {
    let transport = tcp(443, 52_000, 5, 6, TCP_ACK, 512);
    let mut frame = ethernet(LOCAL_MAC, GATEWAY_MAC, ETHERTYPE_IPV4);
    frame.extend_from_slice(&ipv4(
        REMOTE_IPV4,
        LOCAL_IPV4,
        IP_PROTO_TCP,
        transport.len(),
        56,
    ));
    frame.extend_from_slice(&transport);

    let decoded = parse::decode(&frame, &local_context()).expect("frame should decode");
    assert_eq!(decoded.direction, Direction::In);
}

#[test]
fn falls_back_to_private_address_heuristic_without_context() {
    let frame = ipv4_tcp_frame(52_000, 443, TCP_SYN, &[]);
    let decoded = parse::decode(&frame, &DecodeContext::default()).expect("frame should decode");
    // 192.168.1.20 → 93.184.216.34: private source, public destination ⇒ outbound.
    assert_eq!(decoded.direction, Direction::Out);
}

#[test]
fn truncated_and_malformed_frames_never_panic() {
    let context = local_context();
    let frame = ipv4_tcp_frame(52_000, 443, TCP_SYN, &[0x01, 0x02, 0x03]);

    // Every prefix of a valid frame must either decode or return None.
    for length in 0..frame.len() {
        let _ = parse::decode(&frame[..length], &context);
    }
    // Random bytes with a plausible EtherType.
    let mut garbage = ethernet(GATEWAY_MAC, LOCAL_MAC, ETHERTYPE_IPV4);
    garbage.extend_from_slice(&[0xff; 40]);
    let _ = parse::decode(&garbage, &context);
}
