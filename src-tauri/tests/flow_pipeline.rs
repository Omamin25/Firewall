//! End-to-end check of the visibility path *after* decode: real Ethernet frames →
//! flow aggregation → attribution → Connections DTO → `flows` history table.
//!
//! This covers everything the Phase 3 exit criteria need except the live Npcap read,
//! which requires Administrator rights (see `progress_tracker.md`).

mod fixtures;

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use chrono::{DateTime, Duration, Utc};
use cognton_lib::{
    attribution::{
        AttributionBackend, AttributionError, AttributionResolver, ConnectionOwner, ProcessInfo,
    },
    flow::{FlowHistoryWriter, FlowTracker},
    ipc::dto::FlowDto,
    model::{Packet, TransportProtocol},
    parse::{self, DecodeContext},
    store::Store,
};
use fixtures::{l7 as payloads, *};

const CLIENT_PORT: u16 = 51_000;
const OWNER_PID: u32 = 4242;

/// Reports one TCP connection from the client port, owned by a known process.
struct StubBackend;

impl AttributionBackend for StubBackend {
    fn connections(&self) -> Result<Vec<ConnectionOwner>, AttributionError> {
        Ok(vec![ConnectionOwner {
            protocol: TransportProtocol::Tcp,
            local_ip: IpAddr::V4(Ipv4Addr::from(LOCAL_IPV4)),
            local_port: CLIENT_PORT,
            remote: Some((IpAddr::V4(Ipv4Addr::from(REMOTE_IPV4)), 443)),
            pid: OWNER_PID,
        }])
    }

    fn process(&self, pid: u32) -> Result<ProcessInfo, AttributionError> {
        assert_eq!(pid, OWNER_PID);
        Ok(ProcessInfo {
            name: Some("firefox.exe".to_owned()),
            path: Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".to_owned()),
            user_account: Some(r"COGNTON\vraj".to_owned()),
            user_sid: Some("S-1-5-21-1001".to_owned()),
        })
    }
}

fn tracker() -> FlowTracker {
    let backend: Arc<dyn AttributionBackend> = Arc::new(StubBackend);
    FlowTracker::new(Arc::new(AttributionResolver::with_backend(backend)))
}

fn context() -> DecodeContext {
    DecodeContext {
        local_macs: vec![LOCAL_MAC],
        local_ips: vec![IpAddr::V4(Ipv4Addr::from(LOCAL_IPV4))],
    }
}

/// Decodes a frame the way the pipeline does, so the flow key comes from real parsing.
/// Timestamps hang off a wall-clock base: retirement compares them against `Utc::now()`,
/// so epoch-era fixtures would look decades idle.
fn packet(id: u64, base: DateTime<Utc>, offset_secs: i64, frame: &[u8]) -> Packet {
    let decoded = parse::decode(frame, &context()).expect("the fixture frame should decode");
    Packet {
        id,
        timestamp: base + Duration::seconds(offset_secs),
        interface: "test".to_owned(),
        direction: decoded.direction,
        protocol: decoded.protocol,
        remote_host: decoded.remote_host,
        frame_len: frame.len(),
        captured_len: frame.len(),
        layers: decoded.layers,
        flow_id: None,
    }
}

/// The client hello outbound, then a response inbound on the same conversation.
fn conversation(base: DateTime<Utc>) -> Vec<Packet> {
    let hello = ipv4_tcp_frame(
        CLIENT_PORT,
        443,
        TCP_PSH | TCP_ACK,
        &payloads::tls_client_hello_example_com(),
    );
    let reply = ipv4_tcp_reply_frame(
        443,
        CLIENT_PORT,
        TCP_PSH | TCP_ACK,
        &payloads::tls_application_data(),
    );
    vec![packet(1, base, 0, &hello), packet(2, base, 2, &reply)]
}

#[test]
fn both_halves_of_a_tls_conversation_become_one_attributed_flow() {
    let tracker = tracker();
    for mut frame in conversation(Utc::now()) {
        tracker.observe(&mut frame, false);
    }

    let flows = tracker.snapshot();
    assert_eq!(flows.len(), 1, "the two directions must share one flow");
    let flow = &flows[0];
    assert_eq!(flow.attribution.pid, Some(OWNER_PID));
    assert_eq!(
        flow.attribution.process_name.as_deref(),
        Some("firefox.exe")
    );
    assert_eq!(
        flow.attribution.user_account.as_deref(),
        Some(r"COGNTON\vraj")
    );
    assert_eq!(flow.remote_host.as_deref(), Some("example.com"));
    assert!(flow.bytes_sent > 0 && flow.bytes_recv > 0);
    assert_eq!(flow.packets, 2);
    assert_eq!(flow.duration_ms(), 2_000);
}

#[test]
fn packets_are_stamped_with_their_flow_so_the_inspector_can_find_the_owner() {
    let tracker = tracker();
    let mut frames = conversation(Utc::now());
    let owner = tracker.observe(&mut frames[0], true);

    let flow_id = frames[0].flow_id.expect("a TCP frame belongs to a flow");
    assert_eq!(owner.and_then(|owner| owner.pid), Some(OWNER_PID));
    assert_eq!(
        tracker.attribution_of(flow_id).and_then(|owner| owner.pid),
        Some(OWNER_PID)
    );
}

#[test]
fn the_connections_dto_carries_what_the_table_renders() {
    let tracker = tracker();
    for mut frame in conversation(Utc::now()) {
        tracker.observe(&mut frame, false);
    }

    let flow = tracker.snapshot().pop().expect("one flow was recorded");
    let dto = FlowDto::from_flow(&flow);
    assert_eq!(dto.transport, "tcp");
    assert_eq!(dto.protocol, "TLS");
    assert_eq!(dto.protocol_token, "tls");
    assert_eq!(dto.direction, "out");
    assert_eq!(dto.local_address, "192.168.1.20");
    assert_eq!(dto.local_port, Some(CLIENT_PORT));
    assert_eq!(dto.remote_address, "93.184.216.34");
    assert_eq!(dto.remote_port, Some(443));
    assert_eq!(dto.remote_host.as_deref(), Some("example.com"));
    assert_eq!(dto.process_name.as_deref(), Some("firefox.exe"));
    assert_eq!(dto.user_account.as_deref(), Some(r"COGNTON\vraj"));
    assert_eq!(dto.user_sid.as_deref(), Some("S-1-5-21-1001"));
    assert!(dto.active);
}

#[test]
fn a_long_lived_flow_updates_one_history_row_and_closes_on_a_fin() {
    let directory = tempfile::tempdir().expect("test temp directory should be created");
    let store = Arc::new(Store::open(directory.path()).expect("store should open"));
    let tracker = Arc::new(tracker());
    let mut history = FlowHistoryWriter::new(Arc::clone(&store), Arc::clone(&tracker));
    let base = Utc::now();

    for mut frame in conversation(base) {
        tracker.observe(&mut frame, false);
    }
    // The first write inserts the row.
    history.flush(Utc::now());
    assert_eq!(
        store
            .flow_history_summary()
            .expect("summary should load")
            .stored_flows,
        1
    );

    // More traffic on the same conversation, this time a teardown.
    let teardown = ipv4_tcp_frame(CLIENT_PORT, 443, TCP_FIN | TCP_ACK, &[]);
    let mut frame = packet(3, base, 4, &teardown);
    tracker.observe(&mut frame, false);
    history.flush(Utc::now());

    let summary = store.flow_history_summary().expect("summary should load");
    assert_eq!(
        summary.stored_flows, 1,
        "a long-lived flow must update its row, not insert a second one"
    );
    let flow = tracker
        .snapshot()
        .pop()
        .expect("the flow lingers briefly after the FIN");
    assert!(flow.closed, "the FIN should mark the flow closed");
    assert_eq!(flow.packets, 3);
    // The bare ACK/FIN must not relabel a classified conversation.
    assert_eq!(flow.protocol.as_str(), "TLS");
}

#[test]
fn frames_with_no_ip_conversation_are_not_aggregated() {
    let tracker = tracker();
    let mut frame = packet(
        1,
        Utc::now(),
        0,
        &arp(1, LOCAL_MAC, LOCAL_IPV4, GATEWAY_MAC, REMOTE_IPV4),
    );

    assert!(tracker.observe(&mut frame, true).is_none());
    assert_eq!(frame.flow_id, None);
    assert!(tracker.snapshot().is_empty());
}
