//! L7 classifier tests: DNS, HTTP, TLS SNI, QUIC, DHCP, mDNS and the opaque fallback.

mod fixtures;

use cognton_lib::{
    model::{L7Detail, L7Protocol},
    parse::{self, DecodeContext},
};
use fixtures::{l7 as payloads, *};

fn context() -> DecodeContext {
    DecodeContext {
        local_macs: vec![LOCAL_MAC],
        local_ips: vec![std::net::IpAddr::V4(std::net::Ipv4Addr::new(
            192, 168, 1, 20,
        ))],
    }
}

/// Decodes a UDP frame and returns its L7 detail.
fn classify_udp(source_port: u16, dest_port: u16, payload: &[u8]) -> (L7Protocol, L7Detail) {
    let frame = ipv4_udp_frame(source_port, dest_port, payload);
    let decoded = parse::decode(&frame, &context()).expect("frame should decode");
    (
        decoded.protocol,
        decoded.layers.l7.expect("L7 detail should be present"),
    )
}

/// Decodes a TCP frame and returns its L7 detail.
fn classify_tcp(source_port: u16, dest_port: u16, payload: &[u8]) -> (L7Protocol, L7Detail) {
    let frame = ipv4_tcp_frame(source_port, dest_port, TCP_PSH | TCP_ACK, payload);
    let decoded = parse::decode(&frame, &context()).expect("frame should decode");
    (
        decoded.protocol,
        decoded.layers.l7.expect("L7 detail should be present"),
    )
}

#[test]
fn classifies_dns_query() {
    let (protocol, detail) = classify_udp(51_000, 53, &payloads::dns_query_example_com());
    assert_eq!(protocol, L7Protocol::Dns);
    match detail {
        L7Detail::Dns {
            is_response,
            questions,
            answers,
        } => {
            assert!(!is_response);
            assert_eq!(questions, vec!["example.com".to_owned()]);
            assert!(answers.is_empty());
        }
        other => panic!("expected DNS, got {other:?}"),
    }
}

#[test]
fn classifies_dns_response_with_compressed_answer_name() {
    let (protocol, detail) = classify_udp(53, 51_000, &payloads::dns_response_example_com());
    assert_eq!(protocol, L7Protocol::Dns);
    match detail {
        L7Detail::Dns {
            is_response,
            questions,
            answers,
        } => {
            assert!(is_response);
            assert_eq!(questions, vec!["example.com".to_owned()]);
            assert_eq!(answers, vec!["example.com A 93.184.216.34".to_owned()]);
        }
        other => panic!("expected DNS, got {other:?}"),
    }
}

#[test]
fn dns_remote_host_is_exposed_for_the_traffic_table() {
    let frame = ipv4_udp_frame(51_000, 53, &payloads::dns_query_example_com());
    let decoded = parse::decode(&frame, &context()).expect("frame should decode");
    assert_eq!(decoded.remote_host.as_deref(), Some("example.com"));
}

#[test]
fn classifies_mdns_query() {
    let (protocol, detail) = classify_udp(5_353, 5_353, &payloads::mdns_query());
    assert_eq!(protocol, L7Protocol::Mdns);
    match detail {
        L7Detail::Mdns {
            is_response,
            questions,
        } => {
            assert!(!is_response);
            assert_eq!(questions, vec!["_services._dns-sd._udp.local".to_owned()]);
        }
        other => panic!("expected mDNS, got {other:?}"),
    }
}

#[test]
fn classifies_http_request_with_host_header() {
    let (protocol, detail) = classify_tcp(52_100, 80, &payloads::http_get_request());
    assert_eq!(protocol, L7Protocol::Http);
    match detail {
        L7Detail::Http {
            method,
            host,
            path,
            status,
        } => {
            assert_eq!(method.as_deref(), Some("GET"));
            assert_eq!(host.as_deref(), Some("example.com"));
            assert_eq!(path.as_deref(), Some("/index.html"));
            assert_eq!(status, None);
        }
        other => panic!("expected HTTP, got {other:?}"),
    }
}

#[test]
fn classifies_http_status_line() {
    let (protocol, detail) = classify_tcp(80, 52_100, &payloads::http_response_200());
    assert_eq!(protocol, L7Protocol::Http);
    match detail {
        L7Detail::Http { status, method, .. } => {
            assert_eq!(status, Some(200));
            assert_eq!(method, None);
        }
        other => panic!("expected HTTP, got {other:?}"),
    }
}

#[test]
fn extracts_tls_sni_from_client_hello() {
    let (protocol, detail) = classify_tcp(52_200, 443, &payloads::tls_client_hello_example_com());
    assert_eq!(protocol, L7Protocol::Tls);
    match detail {
        L7Detail::Tls {
            sni,
            version,
            handshake,
        } => {
            assert_eq!(sni.as_deref(), Some("example.com"));
            assert_eq!(version.as_deref(), Some("TLS 1.0"));
            assert_eq!(handshake, "ClientHello");
        }
        other => panic!("expected TLS, got {other:?}"),
    }
}

#[test]
fn tls_sni_becomes_the_remote_host() {
    let frame = ipv4_tcp_frame(
        52_200,
        443,
        TCP_PSH | TCP_ACK,
        &payloads::tls_client_hello_example_com(),
    );
    let decoded = parse::decode(&frame, &context()).expect("frame should decode");
    assert_eq!(decoded.remote_host.as_deref(), Some("example.com"));
}

#[test]
fn classifies_tls_application_data_without_sni() {
    let (protocol, detail) = classify_tcp(52_200, 443, &payloads::tls_application_data());
    assert_eq!(protocol, L7Protocol::Tls);
    match detail {
        L7Detail::Tls { sni, handshake, .. } => {
            assert_eq!(sni, None);
            assert_eq!(handshake, "Application Data");
        }
        other => panic!("expected TLS, got {other:?}"),
    }
}

#[test]
fn classifies_quic_long_header_initial() {
    let (protocol, detail) = classify_udp(52_300, 443, &payloads::quic_initial());
    assert_eq!(protocol, L7Protocol::Quic);
    match detail {
        L7Detail::Quic { long_header, sni } => {
            assert!(long_header);
            // SNI lives inside an encrypted CRYPTO frame; see parse::l7::quic.
            assert_eq!(sni, None);
        }
        other => panic!("expected QUIC, got {other:?}"),
    }
}

#[test]
fn classifies_quic_short_header_on_quic_port() {
    let (protocol, detail) = classify_udp(52_300, 443, &payloads::quic_short_header());
    assert_eq!(protocol, L7Protocol::Quic);
    match detail {
        L7Detail::Quic { long_header, .. } => assert!(!long_header),
        other => panic!("expected QUIC, got {other:?}"),
    }
}

#[test]
fn does_not_claim_unrelated_udp_as_quic() {
    let (protocol, detail) = classify_udp(40_000, 40_001, &[0x40; 64]);
    assert_eq!(protocol, L7Protocol::Udp);
    assert!(matches!(detail, L7Detail::Opaque { .. }));
}

#[test]
fn classifies_dhcp_discover_and_ack() {
    let (protocol, detail) = classify_udp(68, 67, &payloads::dhcp_discover());
    assert_eq!(protocol, L7Protocol::Dhcp);
    match detail {
        L7Detail::Dhcp { message_type } => assert_eq!(message_type, "DHCP Discover"),
        other => panic!("expected DHCP, got {other:?}"),
    }

    let (_, detail) = classify_udp(67, 68, &payloads::dhcp_ack());
    match detail {
        L7Detail::Dhcp { message_type } => assert_eq!(message_type, "DHCP ACK"),
        other => panic!("expected DHCP, got {other:?}"),
    }
}

#[test]
fn falls_back_to_opaque_with_a_port_hint() {
    let (protocol, detail) = classify_tcp(52_400, 9_999, &[0x01, 0x02, 0x03, 0x04]);
    assert_eq!(protocol, L7Protocol::Tcp);
    match detail {
        L7Detail::Opaque { hint } => assert_eq!(hint, "TCP port 9999"),
        other => panic!("expected opaque fallback, got {other:?}"),
    }
}

#[test]
fn payload_free_tcp_control_frames_are_labelled_by_flags() {
    let frame = ipv4_tcp_frame(52_500, 443, TCP_SYN, &[]);
    let decoded = parse::decode(&frame, &context()).expect("frame should decode");
    match decoded.layers.l7.expect("L7 detail should be present") {
        L7Detail::Opaque { hint } => assert_eq!(hint, "TCP [SYN]"),
        other => panic!("expected opaque fallback, got {other:?}"),
    }
}

#[test]
fn malformed_l7_payloads_never_panic() {
    // Truncated versions of each real payload must degrade, not crash.
    let payloads = [
        payloads::dns_response_example_com(),
        payloads::tls_client_hello_example_com(),
        payloads::http_get_request(),
        payloads::quic_initial(),
        payloads::dhcp_discover(),
        payloads::mdns_query(),
    ];
    for payload in payloads {
        for length in 0..payload.len() {
            let slice = &payload[..length];
            let _ = classify_udp(53, 51_000, slice);
            let _ = classify_udp(5_353, 5_353, slice);
            let _ = classify_udp(68, 67, slice);
            let _ = classify_udp(52_300, 443, slice);
            let _ = classify_tcp(52_200, 443, slice);
            let _ = classify_tcp(52_100, 80, slice);
        }
    }
}

#[test]
fn dns_compression_pointer_loops_are_rejected() {
    // A pointer that targets itself would loop forever without the backward-only rule.
    let mut message = vec![
        0x12, 0x34, // id
        0x01, 0x00, // flags
        0x00, 0x01, // one question
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    message.extend_from_slice(&[0xc0, 0x0c]); // pointer to offset 12 (itself)
    let (_, detail) = classify_udp(51_000, 53, &message);
    assert!(matches!(detail, L7Detail::Opaque { .. }));
}
