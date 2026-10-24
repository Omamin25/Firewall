//! Engine tests driven by a fake backend, so the whole enforcement path — verdicts,
//! blocked-flow marking, hit persistence, handle lifecycle — is covered without the
//! WinDivert driver.

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{atomic::Ordering, mpsc},
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use etherparse::PacketBuilder;

use super::*;
use crate::{
    attribution::{
        AttributionBackend, AttributionError, AttributionResolver, ConnectionOwner, ProcessInfo,
    },
    flow::FlowTracker,
    model::{
        Direction, Packet, RuleAction, RuleDirection, RuleDraft, RuleProtocol, TransportProtocol,
    },
    parse::{self, DecodeContext},
    rules::RuleSet,
    store::Store,
};

const LOCAL: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 20);
const REMOTE: Ipv4Addr = Ipv4Addr::new(93, 184, 216, 34);

/// Replays a fixed script of packets and socket events, then waits for the stop flag so
/// the engine's lifecycle can be exercised.
struct FakeBackend {
    packets: Vec<(Vec<u8>, bool)>,
    events: Vec<SocketEvent>,
    /// Every verdict the engine returned, in order.
    verdicts: mpsc::Sender<Decision>,
    /// Set when `run_network` refuses to open, standing in for a missing driver.
    fail_network: bool,
}

impl EnforcementBackend for FakeBackend {
    fn run_network(
        &self,
        decide: Decider,
        ready: Ready,
        stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError> {
        if self.fail_network {
            return Err(EnforcementError::Open("no driver in the test".to_owned()));
        }
        ready();
        for (packet, outbound) in &self.packets {
            let verdict = PacketMeta::decode(packet, *outbound)
                .map(|meta| decide(&meta))
                .unwrap_or(Decision::Reinject);
            let _ = self.verdicts.send(verdict);
        }
        wait_for_stop(&stop);
        Ok(())
    }

    fn run_socket(
        &self,
        observe: Observer,
        ready: Ready,
        stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError> {
        ready();
        for event in &self.events {
            observe(*event);
        }
        wait_for_stop(&stop);
        Ok(())
    }
}

fn wait_for_stop(stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(2));
    }
}

/// Connection table that owns the fixture's local endpoint.
struct FakeAttribution;

impl AttributionBackend for FakeAttribution {
    fn connections(&self) -> Result<Vec<ConnectionOwner>, AttributionError> {
        Ok(vec![ConnectionOwner {
            protocol: TransportProtocol::Tcp,
            local_ip: IpAddr::V4(LOCAL),
            local_port: 51_000,
            remote: Some((IpAddr::V4(REMOTE), 443)),
            pid: 4_242,
        }])
    }

    fn process(&self, pid: u32) -> Result<ProcessInfo, AttributionError> {
        Ok(ProcessInfo {
            name: Some("telemetry.exe".to_owned()),
            path: Some(format!(r"C:\Apps\{pid}\telemetry.exe")),
            user_account: Some(r"OMEN\vraj".to_owned()),
            user_sid: Some("S-1-5-21-7-1001".to_owned()),
        })
    }
}

fn tcp_packet(remote_port: u16) -> Vec<u8> {
    let builder = PacketBuilder::ipv4(LOCAL.octets(), REMOTE.octets(), 64).tcp(
        51_000,
        remote_port,
        1,
        64_240,
    );
    let mut bytes = Vec::with_capacity(builder.size(4));
    builder
        .write(&mut bytes, &[1, 2, 3, 4])
        .expect("fixture should serialize");
    bytes
}

/// The same conversation as an Ethernet frame, so the capture path creates the flow that
/// enforcement then marks.
fn tcp_frame(remote_port: u16) -> Vec<u8> {
    let builder = PacketBuilder::ethernet2([2, 0, 0, 0, 0, 1], [2, 0, 0, 0, 0, 2])
        .ipv4(LOCAL.octets(), REMOTE.octets(), 64)
        .tcp(51_000, remote_port, 1, 64_240);
    let mut bytes = Vec::with_capacity(builder.size(4));
    builder
        .write(&mut bytes, &[1, 2, 3, 4])
        .expect("fixture should serialize");
    bytes
}

fn draft(name: &str, action: RuleAction) -> RuleDraft {
    RuleDraft {
        name: name.to_owned(),
        action,
        direction: RuleDirection::Both,
        protocol: RuleProtocol::Any,
        remote_ip: None,
        remote_port: None,
        local_port: None,
        process_path: None,
        user_sid: None,
        enabled: true,
        priority: 100,
    }
}

struct Harness {
    _directory: tempfile::TempDir,
    engine: Arc<EnforcementEngine>,
    rules: Arc<RuleSet>,
    tracker: Arc<FlowTracker>,
    store: Arc<Store>,
    verdicts: mpsc::Receiver<Decision>,
}

impl Harness {
    fn new(backend: impl FnOnce(mpsc::Sender<Decision>) -> FakeBackend) -> Self {
        let directory = tempfile::tempdir().expect("test temp directory should be created");
        let store = Arc::new(Store::open(directory.path()).expect("store should open"));
        let resolver = Arc::new(AttributionResolver::with_backend(Arc::new(FakeAttribution)));
        let tracker = Arc::new(FlowTracker::new(Arc::clone(&resolver)));
        let rules = Arc::new(RuleSet::default());
        let (sender, verdicts) = mpsc::channel();
        let engine = Arc::new(EnforcementEngine::new(
            Arc::new(backend(sender)),
            Arc::clone(&rules),
            resolver,
            Arc::clone(&tracker),
            Arc::clone(&store),
        ));
        Self {
            _directory: directory,
            engine,
            rules,
            tracker,
            store,
            verdicts,
        }
    }

    /// Loads rules from the database into the hot set and brings the handle in line, the
    /// same sequence the IPC commands use.
    fn reload(&self) {
        let stored = self.store.list_rules().expect("rules should load");
        assert!(
            self.rules.load(&stored).is_empty(),
            "fixtures should compile"
        );
        self.engine.reconcile();
    }

    fn next_verdict(&self) -> Decision {
        self.verdicts
            .recv_timeout(Duration::from_secs(5))
            .expect("the backend should have reported a verdict")
    }

    /// Folds a frame through the capture path so a flow exists to be marked.
    fn observe_frame(&self, remote_port: u16) {
        let frame = tcp_frame(remote_port);
        let context = DecodeContext {
            local_macs: Vec::new(),
            local_ips: vec![IpAddr::V4(LOCAL)],
        };
        let decoded = parse::decode(&frame, &context).expect("fixture should decode");
        let mut packet = Packet {
            id: 1,
            timestamp: Utc::now(),
            interface: "test".to_owned(),
            direction: decoded.direction,
            protocol: decoded.protocol,
            remote_host: decoded.remote_host,
            frame_len: frame.len(),
            captured_len: frame.len(),
            layers: decoded.layers,
            flow_id: None,
        };
        assert_eq!(packet.direction, Direction::Out);
        self.tracker.observe(&mut packet, false);
    }

    fn wait_until(&self, mut condition: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if condition() {
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.engine.stop();
    }
}

fn backend(
    packets: Vec<(Vec<u8>, bool)>,
    events: Vec<SocketEvent>,
) -> impl FnOnce(mpsc::Sender<Decision>) -> FakeBackend {
    move |verdicts| FakeBackend {
        packets,
        events,
        verdicts,
        fail_network: false,
    }
}

#[test]
fn no_block_rules_means_no_network_handle() {
    let harness = Harness::new(backend(vec![(tcp_packet(443), true)], Vec::new()));
    harness.engine.start();

    // The socket listener still runs — attribution wants it regardless of any rule.
    assert!(harness.wait_until(|| harness.engine.status().observing));
    assert!(!harness.engine.status().enforcing);

    // An allow-only rule set must not open the handle either.
    harness
        .store
        .create_rule(&draft("allow all", RuleAction::Allow), Utc::now())
        .expect("rule should save");
    harness.reload();
    assert!(!harness.engine.status().enforcing);
}

#[test]
fn a_block_rule_opens_the_handle_and_drops_matching_packets() {
    let harness = Harness::new(backend(vec![(tcp_packet(443), true)], Vec::new()));
    harness.engine.start();
    harness.observe_frame(443);

    let mut rule = draft("block https", RuleAction::Block);
    rule.remote_port = Some(443);
    let saved = harness
        .store
        .create_rule(&rule, Utc::now())
        .expect("rule should save");
    harness.reload();

    assert_eq!(harness.next_verdict(), Decision::Drop);
    assert!(harness.wait_until(|| harness.engine.status().blocked_packets == 1));
    assert!(harness.engine.status().enforcing);

    // The flow the capture path created is marked with the rule that stopped it.
    let flows = harness.tracker.snapshot();
    let flow = flows.first().expect("the frame should have made a flow");
    assert!(flow.blocked);
    assert_eq!(
        flow.matched_rule.as_ref().map(|matched| matched.id),
        Some(saved.id)
    );
}

#[test]
fn a_packet_no_rule_names_is_reinjected() {
    let harness = Harness::new(backend(vec![(tcp_packet(80), true)], Vec::new()));
    harness.engine.start();

    let mut rule = draft("block https only", RuleAction::Block);
    rule.remote_port = Some(443);
    harness
        .store
        .create_rule(&rule, Utc::now())
        .expect("rule should save");
    harness.reload();

    assert_eq!(harness.next_verdict(), Decision::Reinject);
    assert_eq!(harness.engine.status().blocked_packets, 0);
}

#[test]
fn a_process_rule_resolves_the_owner_before_deciding() {
    let harness = Harness::new(backend(vec![(tcp_packet(443), true)], Vec::new()));
    harness.engine.start();

    let mut rule = draft("block telemetry", RuleAction::Block);
    rule.process_path = Some("telemetry.exe".to_owned());
    harness
        .store
        .create_rule(&rule, Utc::now())
        .expect("rule should save");
    harness.reload();

    assert!(harness.rules.needs_attribution());
    assert_eq!(harness.next_verdict(), Decision::Drop);
}

#[test]
fn toggling_a_rule_off_closes_the_handle() {
    let harness = Harness::new(backend(Vec::new(), Vec::new()));
    harness.engine.start();

    let saved = harness
        .store
        .create_rule(&draft("block all", RuleAction::Block), Utc::now())
        .expect("rule should save");
    harness.reload();
    assert!(harness.wait_until(|| harness.engine.status().enforcing));

    harness
        .store
        .set_rule_enabled(saved.id, false, Utc::now())
        .expect("rule should toggle");
    harness.reload();
    assert!(!harness.engine.status().enforcing);
    assert_eq!(harness.engine.status().block_rules, 0);
}

#[test]
fn socket_events_publish_and_retire_pid_hints() {
    let opened = SocketEvent {
        kind: SocketEventKind::Opened,
        pid: 777,
        protocol: TransportProtocol::Udp,
        local_ip: IpAddr::V4(LOCAL),
        local_port: 53_000,
    };
    let harness = Harness::new(backend(Vec::new(), vec![opened]));
    harness.engine.start();

    assert!(harness.wait_until(|| harness.engine.status().tracked_endpoints == 1));
    assert_eq!(harness.engine.status().socket_events, 1);
}

#[test]
fn a_driver_that_will_not_open_surfaces_an_error_and_keeps_traffic_flowing() {
    let harness = Harness::new(|verdicts| FakeBackend {
        packets: Vec::new(),
        events: Vec::new(),
        verdicts,
        fail_network: true,
    });
    harness.engine.start();

    harness
        .store
        .create_rule(&draft("block all", RuleAction::Block), Utc::now())
        .expect("rule should save");
    harness.reload();

    assert!(harness.wait_until(|| harness.engine.status().error.is_some()));
    let status = harness.engine.status();
    assert!(!status.enforcing);
    assert!(status
        .error
        .expect("an error should be recorded")
        .contains("WinDivert"));
}

#[test]
fn hits_reach_the_database_when_the_engine_stops() {
    let harness = Harness::new(backend(
        vec![(tcp_packet(443), true), (tcp_packet(443), true)],
        Vec::new(),
    ));
    harness.engine.start();

    let mut rule = draft("block https", RuleAction::Block);
    rule.remote_port = Some(443);
    let saved = harness
        .store
        .create_rule(&rule, Utc::now())
        .expect("rule should save");
    harness.reload();

    assert_eq!(harness.next_verdict(), Decision::Drop);
    assert_eq!(harness.next_verdict(), Decision::Drop);
    assert!(harness.wait_until(|| harness.rules.hit_counts() == vec![(saved.id, 2)]));

    harness.engine.stop();
    let stored = harness
        .store
        .get_rule(saved.id)
        .expect("lookup should succeed")
        .expect("rule should exist");
    assert_eq!(stored.hit_count, 2);
}

/// Guards the choice to keep the two engines independent: enforcement must not invent
/// connections the visibility engine has never seen (`architecture.md` §5 rule 4).
#[test]
fn blocking_a_flow_capture_never_saw_creates_no_row() {
    let harness = Harness::new(backend(vec![(tcp_packet(443), true)], Vec::new()));
    harness.engine.start();

    harness
        .store
        .create_rule(&draft("block all", RuleAction::Block), Utc::now())
        .expect("rule should save");
    harness.reload();

    assert_eq!(harness.next_verdict(), Decision::Drop);
    assert!(harness.tracker.snapshot().is_empty());
}
