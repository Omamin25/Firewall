use std::net::{IpAddr, Ipv4Addr};

use chrono::{TimeZone, Utc};

use super::*;
use crate::model::{
    layers::{L2Detail, L4Detail, PacketLayer, TcpFlags},
    Direction, L7Protocol,
};

const LOCAL: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 20);
const REMOTE: Ipv4Addr = Ipv4Addr::new(93, 184, 216, 34);

/// Builds a TCP frame at `second` past the epoch. Only the fields flow aggregation
/// reads are populated.
fn tcp_packet(
    id: u64,
    second: i64,
    direction: Direction,
    bytes: usize,
    protocol: L7Protocol,
    flags: TcpFlags,
) -> Packet {
    let (source_port, dest_port) = match direction {
        Direction::Out => (51_000, 443),
        Direction::In => (443, 51_000),
    };
    let (source, dest) = match direction {
        Direction::Out => (LOCAL, REMOTE),
        Direction::In => (REMOTE, LOCAL),
    };
    Packet {
        id,
        timestamp: Utc
            .timestamp_opt(second, 0)
            .single()
            .unwrap_or_else(Utc::now),
        interface: "test".to_owned(),
        direction,
        protocol,
        remote_host: None,
        frame_len: bytes,
        captured_len: bytes,
        layers: PacketLayer {
            l2: L2Detail {
                source_mac: [0; 6],
                dest_mac: [0; 6],
                ether_type: 0x0800,
            },
            l3: Some(crate::model::layers::L3Detail::Ipv4 {
                source,
                dest,
                ttl: 64,
                protocol: 6,
                dont_fragment: true,
                more_fragments: false,
                fragment_offset: 0,
            }),
            l4: Some(L4Detail::Tcp {
                source_port,
                dest_port,
                sequence: 0,
                acknowledgment: 0,
                flags,
                window: 512,
            }),
            l7: None,
            payload_len: 0,
        },
        flow_id: None,
    }
}
fn outbound(id: u64, second: i64, bytes: usize) -> Packet {
    tcp_packet(
        id,
        second,
        Direction::Out,
        bytes,
        L7Protocol::Tcp,
        TcpFlags::default(),
    )
}

fn inbound(id: u64, second: i64, bytes: usize) -> Packet {
    tcp_packet(
        id,
        second,
        Direction::In,
        bytes,
        L7Protocol::Tcp,
        TcpFlags::default(),
    )
}

fn at(second: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(second, 0)
        .single()
        .unwrap_or_else(Utc::now)
}

fn record(table: &mut FlowTable, packet: &Packet) -> Recorded {
    let key = FlowKey::from_packet(packet).expect("a TCP/IPv4 frame has a flow key");
    table.record(&key, packet, false)
}

#[test]
fn both_directions_of_a_conversation_share_one_flow() {
    let mut table = FlowTable::new(16);
    let first = record(&mut table, &outbound(1, 100, 200));
    let second = record(&mut table, &inbound(2, 101, 1_400));

    assert_eq!(first.flow_id, second.flow_id);
    assert_eq!(table.len(), 1);
}

#[test]
fn counters_split_bytes_by_direction() {
    let mut table = FlowTable::new(16);
    record(&mut table, &outbound(1, 100, 200));
    record(&mut table, &outbound(2, 101, 300));
    record(&mut table, &inbound(3, 102, 1_400));

    let flow = table.snapshot().pop().expect("one flow was recorded");
    assert_eq!(flow.bytes_sent, 500);
    assert_eq!(flow.bytes_recv, 1_400);
    assert_eq!(flow.bytes_total(), 1_900);
    assert_eq!(flow.packets, 3);
    assert_eq!(flow.direction, Direction::Out);
}

#[test]
fn duration_spans_first_to_last_frame_even_when_they_arrive_out_of_order() {
    let mut table = FlowTable::new(16);
    record(&mut table, &outbound(1, 100, 100));
    record(&mut table, &inbound(2, 130, 100));
    // Interleaved capture threads can deliver an earlier frame later.
    record(&mut table, &outbound(3, 95, 100));

    let flow = table.snapshot().pop().expect("one flow was recorded");
    assert_eq!(flow.first_seen, at(95));
    assert_eq!(flow.last_seen, at(130));
    assert_eq!(flow.duration_ms(), 35_000);
}

#[test]
fn an_application_protocol_sticks_once_classified() {
    let mut table = FlowTable::new(16);
    record(&mut table, &outbound(1, 100, 100));
    record(
        &mut table,
        &tcp_packet(
            2,
            101,
            Direction::Out,
            517,
            L7Protocol::Tls,
            TcpFlags::default(),
        ),
    );
    // A later bare ACK must not relabel the conversation as plain TCP.
    record(&mut table, &inbound(3, 102, 60));

    let flow = table.snapshot().pop().expect("one flow was recorded");
    assert_eq!(flow.protocol, L7Protocol::Tls);
}

#[test]
fn a_fin_marks_the_flow_closed() {
    let mut table = FlowTable::new(16);
    record(&mut table, &outbound(1, 100, 100));
    record(
        &mut table,
        &tcp_packet(
            2,
            101,
            Direction::Out,
            60,
            L7Protocol::Tcp,
            TcpFlags {
                fin: true,
                ack: true,
                ..TcpFlags::default()
            },
        ),
    );

    let flow = table.snapshot().pop().expect("one flow was recorded");
    assert!(flow.closed);
}

#[test]
fn only_changed_flows_are_drained_for_the_next_push() {
    let mut table = FlowTable::new(16);
    record(&mut table, &outbound(1, 100, 100));
    assert_eq!(table.drain_dirty().len(), 1);
    // Nothing changed since the last drain.
    assert!(table.drain_dirty().is_empty());

    record(&mut table, &inbound(2, 101, 100));
    assert_eq!(table.drain_dirty().len(), 1);
}

#[test]
fn a_resolved_owner_marks_the_flow_changed_only_once() {
    let mut table = FlowTable::new(16);
    let packet = outbound(1, 100, 100);
    let key = FlowKey::from_packet(&packet).expect("a TCP/IPv4 frame has a flow key");
    table.record(&key, &packet, false);
    table.drain_dirty();

    let owner = Attribution {
        pid: Some(4242),
        process_name: Some("firefox.exe".to_owned()),
        ..Attribution::default()
    };
    table.attribute(&key, owner.clone(), false);
    assert_eq!(table.drain_dirty().len(), 1);
    // Re-resolving the same owner is not a change.
    table.attribute(&key, owner, false);
    assert!(table.drain_dirty().is_empty());
}

#[test]
fn attribution_attempts_are_capped_per_flow() {
    let mut table = FlowTable::new(16);
    let attempts = (0..20)
        .filter(|index| record(&mut table, &outbound(*index + 1, 100, 100)).wants_attribution)
        .count();
    assert_eq!(attempts, 6);
}

#[test]
fn a_resolved_flow_stops_asking_for_attribution() {
    let mut table = FlowTable::new(16);
    let packet = outbound(1, 100, 100);
    let key = FlowKey::from_packet(&packet).expect("a TCP/IPv4 frame has a flow key");
    table.record(&key, &packet, false);
    table.attribute(&key, Attribution::from_pid(4242), false);

    let next = table.record(&key, &outbound(2, 101, 100), false);
    assert!(!next.wants_attribution);
}

#[test]
fn idle_flows_are_retired_and_owed_a_write() {
    let mut table = FlowTable::new(16).with_windows(Duration::seconds(30), Duration::seconds(5));
    record(&mut table, &outbound(1, 100, 100));

    let quiet = table.maintain(at(120));
    assert!(
        quiet.retired_ids.is_empty(),
        "20s idle is within the window"
    );

    let expired = table.maintain(at(200));
    assert_eq!(expired.retired_ids.len(), 1);
    assert_eq!(expired.persist.len(), 1);
    assert_eq!(expired.persist[0].row_id, None);
    assert!(table.is_empty());
}

#[test]
fn closed_flows_retire_on_the_shorter_window() {
    let mut table = FlowTable::new(16).with_windows(Duration::seconds(300), Duration::seconds(5));
    record(
        &mut table,
        &tcp_packet(
            1,
            100,
            Direction::Out,
            60,
            L7Protocol::Tcp,
            TcpFlags {
                rst: true,
                ..TcpFlags::default()
            },
        ),
    );

    // Well inside the idle timeout, but past the linger window for a torn-down flow.
    let retired = table.maintain(at(110));
    assert_eq!(retired.retired_ids.len(), 1);
}

#[test]
fn the_table_stays_within_its_capacity() {
    let mut table = FlowTable::new(8);
    // Each frame uses a distinct remote port, so each is its own flow.
    for index in 0..40_u16 {
        let mut packet = outbound(u64::from(index) + 1, 100 + i64::from(index), 100);
        if let Some(L4Detail::Tcp { dest_port, .. }) = packet.layers.l4.as_mut() {
            *dest_port = 1_000 + index;
        }
        record(&mut table, &packet);
    }
    assert!(table.len() <= 8, "capacity was exceeded: {}", table.len());

    // Everything evicted is still handed back for the history table.
    let maintenance = table.maintain(at(200));
    assert!(maintenance.retired_ids.len() >= 32);
}

#[test]
fn row_ids_are_recorded_so_later_writes_update_instead_of_insert() {
    let mut table = FlowTable::new(16);
    record(&mut table, &outbound(1, 100, 100));

    let pending = table.take_unsaved();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].row_id, None);
    let flow_id = pending[0].flow.id;
    table.assign_row_ids(&[(flow_id, 77)]);

    // Nothing changed, so nothing is owed.
    assert!(table.take_unsaved().is_empty());

    record(&mut table, &inbound(2, 101, 100));
    let pending = table.take_unsaved();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].row_id, Some(77));
}

#[test]
fn frames_without_an_ip_conversation_have_no_flow_key() {
    let mut packet = outbound(1, 100, 100);
    packet.layers.l4 = None;
    assert!(FlowKey::from_packet(&packet).is_none());

    let mut arp = outbound(2, 100, 60);
    arp.layers.l3 = Some(crate::model::layers::L3Detail::Arp {
        operation: 1,
        sender_ip: LOCAL,
        target_ip: REMOTE,
    });
    assert!(FlowKey::from_packet(&arp).is_none());
}

#[test]
fn the_owner_snapshot_is_only_cloned_when_asked_for() {
    let mut table = FlowTable::new(16);
    let packet = outbound(1, 100, 100);
    let key = FlowKey::from_packet(&packet).expect("a TCP/IPv4 frame has a flow key");
    table.record(&key, &packet, false);
    table.attribute(&key, Attribution::from_pid(9), false);

    assert!(table.record(&key, &packet, false).owner.is_none());
    assert_eq!(
        table
            .record(&key, &packet, true)
            .owner
            .and_then(|owner| owner.pid),
        Some(9)
    );
}

#[test]
fn icmp_flows_are_keyed_without_ports() {
    let mut packet = outbound(1, 100, 84);
    packet.layers.l4 = Some(L4Detail::Icmp { kind: 8, code: 0 });
    let key = FlowKey::from_packet(&packet).expect("an ICMP frame still has a flow key");

    assert_eq!(key.protocol, crate::model::TransportProtocol::Icmp);
    assert_eq!(key.local_port, 0);
    assert_eq!(key.remote_port, 0);
    assert_eq!(key.local_ip, IpAddr::V4(LOCAL));
}
