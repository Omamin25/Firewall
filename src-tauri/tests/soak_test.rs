//! Sustained-traffic soak test (`build_plan.md` Phase 6).
//!
//! Pushes a mixed-protocol packet stream through the full offline pipeline — decode →
//! flow aggregation → attribution lookup → rule evaluation → packet ring — and asserts
//! both throughput and the bounds Phase 6 claims: the ring never exceeds its capacity,
//! the flow table stays within its LRU cap, and rule matching stays correct under load.
//!
//! Everything here is deterministic and needs no elevation: the capture and enforcement
//! drivers are the only parts that do, and they are covered by the smoke examples.

mod fixtures;

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Instant,
};

use chrono::Utc;
use cognton_lib::{
    attribution::AttributionResolver,
    flow::{FlowTable, FlowTracker},
    ipc::ring::PacketRing,
    model::{
        L7Protocol, Packet, Rule, RuleAction, RuleDirection, RuleDraft, RuleProtocol,
        TransportProtocol,
    },
    parse::{self, DecodeContext},
    rules::{Candidate, RuleSet},
};
use fixtures::{l7 as payloads, *};

/// Matches the production ring size in `ipc/mod.rs`.
const RING_CAPACITY: usize = 20_000;
/// The production live-flow cap, referenced rather than duplicated so the bound under
/// test cannot drift away from the one the app actually uses.
const FLOW_CAPACITY: usize = cognton_lib::flow::DEFAULT_CAPACITY;
const PACKET_COUNT: usize = 50_000;

fn context() -> DecodeContext {
    DecodeContext {
        local_macs: vec![LOCAL_MAC],
        local_ips: vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))],
    }
}

/// One frame per protocol the classifiers handle, so the soak stream is genuinely mixed
/// rather than 50,000 copies of the same bare TCP segment.
struct Sample {
    frame: Vec<u8>,
    protocol: L7Protocol,
    transport: TransportProtocol,
    local_port: u16,
    remote_port: u16,
}

fn samples() -> Vec<Sample> {
    vec![
        Sample {
            frame: ipv4_tcp_frame(52_100, 80, TCP_PSH | TCP_ACK, &payloads::http_get_request()),
            protocol: L7Protocol::Http,
            transport: TransportProtocol::Tcp,
            local_port: 52_100,
            remote_port: 80,
        },
        Sample {
            frame: ipv4_tcp_frame(
                52_200,
                443,
                TCP_PSH | TCP_ACK,
                &payloads::tls_client_hello_example_com(),
            ),
            protocol: L7Protocol::Tls,
            transport: TransportProtocol::Tcp,
            local_port: 52_200,
            remote_port: 443,
        },
        Sample {
            frame: ipv4_tcp_frame(52_300, 8_443, TCP_SYN, &[]),
            protocol: L7Protocol::Tcp,
            transport: TransportProtocol::Tcp,
            local_port: 52_300,
            remote_port: 8_443,
        },
        Sample {
            frame: ipv4_udp_frame(51_000, 53, &payloads::dns_query_example_com()),
            protocol: L7Protocol::Dns,
            transport: TransportProtocol::Udp,
            local_port: 51_000,
            remote_port: 53,
        },
        Sample {
            frame: ipv4_udp_frame(51_100, 443, &payloads::quic_initial()),
            protocol: L7Protocol::Quic,
            transport: TransportProtocol::Udp,
            local_port: 51_100,
            remote_port: 443,
        },
        Sample {
            frame: ipv4_udp_frame(5_353, 5_353, &payloads::mdns_query()),
            protocol: L7Protocol::Mdns,
            transport: TransportProtocol::Udp,
            local_port: 5_353,
            remote_port: 5_353,
        },
    ]
}

fn block_http_rule() -> Rule {
    Rule {
        id: 1,
        draft: RuleDraft {
            name: "Block HTTP".to_owned(),
            action: RuleAction::Block,
            direction: RuleDirection::Both,
            protocol: RuleProtocol::Tcp,
            remote_ip: None,
            remote_port: Some(80),
            local_port: None,
            process_path: None,
            user_sid: None,
            enabled: true,
            priority: 100,
        },
        hit_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn sustained_mixed_traffic_stays_within_bounds_and_keeps_up() {
    let resolver = Arc::new(AttributionResolver::new());
    // An explicit table so the cap under test is the production one, not a default that
    // could drift away from it silently.
    let tracker = FlowTracker::with_table(Arc::clone(&resolver), FlowTable::new(FLOW_CAPACITY));
    let ring = PacketRing::new(RING_CAPACITY);
    let context = context();

    let rules = RuleSet::default();
    assert!(
        rules.load(&[block_http_rule()]).is_empty(),
        "the soak rule should compile"
    );

    let samples = samples();
    let sample_count = samples.len();
    let mut blocked = 0_usize;
    let mut decoded_protocols = std::collections::BTreeSet::new();

    let start = Instant::now();

    for index in 0..PACKET_COUNT {
        let sample = &samples[index % samples.len()];
        // Spread traffic over many remote hosts so flow-key churn (and the LRU cap)
        // is genuinely exercised instead of hitting one hot entry.
        let remote_ip = IpAddr::V4(Ipv4Addr::new(93, 184, 216, (index % 251) as u8 + 1));

        let decoded = parse::decode(&sample.frame, &context).expect("fixture frame should decode");
        assert_eq!(
            decoded.protocol, sample.protocol,
            "classifier drifted for {:?}",
            sample.protocol
        );
        decoded_protocols.insert(decoded.protocol.as_str());

        let mut packet = Packet {
            id: index as u64 + 1,
            timestamp: Utc::now(),
            interface: r"\Device\NPF_SOAK".to_owned(),
            direction: decoded.direction,
            protocol: decoded.protocol,
            remote_host: decoded.remote_host,
            frame_len: sample.frame.len(),
            captured_len: sample.frame.len(),
            layers: decoded.layers,
            flow_id: None,
        };

        let owner = tracker.observe(&mut packet, true);
        assert!(
            packet.flow_id.is_some(),
            "every decoded IP packet should join a flow"
        );

        let candidate = Candidate {
            direction: decoded.direction,
            protocol: sample.transport,
            local_port: sample.local_port,
            remote_ip,
            remote_port: sample.remote_port,
            process_path: owner.as_ref().and_then(|o| o.process_path.as_deref()),
            user_sid: owner.as_ref().and_then(|o| o.user_sid.as_deref()),
        };
        let evaluation = rules.evaluate(&candidate);
        if sample.remote_port == 80 && sample.transport == TransportProtocol::Tcp {
            assert!(
                evaluation.is_block(),
                "the port-80 block rule should match every HTTP sample"
            );
            blocked += 1;
        } else {
            assert!(
                !evaluation.is_block(),
                "only HTTP should be blocked, but {:?}:{} matched",
                sample.transport,
                sample.remote_port
            );
        }

        ring.push(packet);

        // The ring is the UI's inspector buffer; exceeding it would mean unbounded
        // memory growth under sustained capture.
        assert!(
            ring.len() <= RING_CAPACITY,
            "packet ring exceeded its capacity at packet {index}"
        );
    }

    let elapsed = start.elapsed();
    let pps = PACKET_COUNT as f64 / elapsed.as_secs_f64();
    println!(
        "soak: {PACKET_COUNT} mixed packets in {elapsed:.2?} ({pps:.0} pkt/s), {blocked} blocked"
    );

    assert_eq!(
        decoded_protocols.len(),
        sample_count,
        "every sampled protocol should have been decoded at least once"
    );
    // The round-robin does not divide evenly, so the first samples get one extra pass.
    let http_passes =
        PACKET_COUNT / sample_count + usize::from(!PACKET_COUNT.is_multiple_of(sample_count));
    assert_eq!(
        blocked, http_passes,
        "each HTTP sample should have been blocked exactly once"
    );
    assert_eq!(
        ring.len(),
        RING_CAPACITY,
        "the ring should be saturated but bounded"
    );
    assert!(
        tracker.active_count() <= FLOW_CAPACITY,
        "live flows ({}) exceeded the LRU cap of {FLOW_CAPACITY}",
        tracker.active_count()
    );
    assert!(
        elapsed.as_millis() < 3_000,
        "{PACKET_COUNT} packets took too long: {elapsed:?}"
    );
}

/// The flow table's LRU cap is what keeps memory bounded when a host talks to a very
/// large number of endpoints, so it gets its own explicit check with unique 5-tuples.
#[test]
fn flow_table_enforces_its_cap_under_unique_flow_churn() {
    let resolver = Arc::new(AttributionResolver::new());
    let tracker = FlowTracker::with_table(resolver, FlowTable::new(FLOW_CAPACITY));
    let context = context();

    // Twice the cap in distinct source ports, so half of them must be evicted.
    for index in 0..(FLOW_CAPACITY * 2) {
        let source_port = 10_000 + (index % 55_000) as u16;
        let frame = ipv4_tcp_frame(source_port, 443, TCP_SYN, &[]);
        let decoded = parse::decode(&frame, &context).expect("frame should decode");
        let mut packet = Packet {
            id: index as u64 + 1,
            timestamp: Utc::now(),
            interface: r"\Device\NPF_SOAK".to_owned(),
            direction: decoded.direction,
            protocol: decoded.protocol,
            remote_host: decoded.remote_host,
            frame_len: frame.len(),
            captured_len: frame.len(),
            layers: decoded.layers,
            flow_id: None,
        };
        // `want_owner: false` keeps this focused on the cap rather than on syscalls.
        tracker.observe(&mut packet, false);
    }

    let live = tracker.active_count();
    assert!(
        live <= FLOW_CAPACITY,
        "live flows ({live}) exceeded the LRU cap of {FLOW_CAPACITY}"
    );
    assert!(
        live > FLOW_CAPACITY / 2,
        "eviction was far too aggressive: only {live} flows survived"
    );
    assert_eq!(
        tracker.snapshot().len(),
        live,
        "the snapshot the UI seeds from should agree with the live count"
    );
}
