use std::net::{IpAddr, Ipv4Addr};

use chrono::{TimeZone, Utc};

use super::*;
use crate::model::{Attribution, Direction, FlowKey, L7Protocol, TransportProtocol};

fn store() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().expect("test temp directory should be created");
    let store = Store::open(directory.path()).expect("store should open");
    (directory, store)
}

fn at(second: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(second, 0)
        .single()
        .unwrap_or_else(Utc::now)
}

/// A flow whose only interesting property is when it was last seen.
fn flow(id: u64, remote_port: u16, last_seen_secs: i64) -> Flow {
    Flow {
        id,
        key: FlowKey {
            protocol: TransportProtocol::Tcp,
            local_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
            local_port: 51_000 + remote_port,
            remote_ip: IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            remote_port,
        },
        first_seen: at(last_seen_secs - 5),
        last_seen: at(last_seen_secs),
        direction: Direction::Out,
        protocol: L7Protocol::Tls,
        remote_host: Some("example.com".to_owned()),
        bytes_sent: 1_200,
        bytes_recv: 34_000,
        packets: 42,
        closed: false,
        attribution: Attribution {
            pid: Some(4242),
            process_name: Some("firefox.exe".to_owned()),
            process_path: Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".to_owned()),
            user_account: Some(r"COGNTON\vraj".to_owned()),
            user_sid: Some("S-1-5-21-1001".to_owned()),
        },
        blocked: false,
        matched_rule: None,
    }
}

fn pending(flow: Flow, row_id: Option<i64>) -> PersistableFlow {
    PersistableFlow { row_id, flow }
}

#[test]
fn a_first_write_inserts_and_returns_its_row_id() {
    let (_dir, store) = store();
    let assigned = store
        .upsert_flows(&[pending(flow(1, 443, 1_000), None)])
        .expect("insert should succeed");

    assert_eq!(assigned.len(), 1);
    assert_eq!(assigned[0].0, 1);
    let summary = store.flow_history_summary().expect("summary should load");
    assert_eq!(summary.stored_flows, 1);
}

#[test]
fn a_later_write_updates_the_same_row() {
    let (_dir, store) = store();
    let assigned = store
        .upsert_flows(&[pending(flow(1, 443, 1_000), None)])
        .expect("insert should succeed");
    let row_id = assigned[0].1;

    let mut grown = flow(1, 443, 1_100);
    grown.bytes_recv = 90_000;
    grown.packets = 120;
    store
        .upsert_flows(&[pending(grown, Some(row_id))])
        .expect("update should succeed");

    let summary = store.flow_history_summary().expect("summary should load");
    assert_eq!(summary.stored_flows, 1, "the flow must not be duplicated");

    let connection = store.connection().expect("connection should lock");
    let (bytes_recv, packets): (i64, i64) = connection
        .query_row(
            "SELECT bytes_recv, packets FROM flows WHERE id = ?1",
            [row_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the row should still exist");
    assert_eq!(bytes_recv, 90_000);
    assert_eq!(packets, 120);
}

#[test]
fn attribution_and_transport_round_trip() {
    let (_dir, store) = store();
    store
        .upsert_flows(&[pending(flow(1, 443, 1_000), None)])
        .expect("insert should succeed");

    let connection = store.connection().expect("connection should lock");
    let (protocol, l7, process, user, sid): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT protocol, l7_protocol, process_path, user_account, user_sid FROM flows",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("the row should exist");

    assert_eq!(protocol, "tcp");
    assert_eq!(l7.as_deref(), Some("TLS"));
    assert_eq!(
        process.as_deref(),
        Some(r"C:\Program Files\Mozilla Firefox\firefox.exe")
    );
    assert_eq!(user.as_deref(), Some(r"COGNTON\vraj"));
    assert_eq!(sid.as_deref(), Some("S-1-5-21-1001"));
}

#[test]
fn an_unclassified_flow_stores_no_application_protocol() {
    let (_dir, store) = store();
    let mut plain = flow(1, 8_443, 1_000);
    plain.protocol = L7Protocol::Tcp;
    store
        .upsert_flows(&[pending(plain, None)])
        .expect("insert should succeed");

    let connection = store.connection().expect("connection should lock");
    let l7: Option<String> = connection
        .query_row("SELECT l7_protocol FROM flows", [], |row| row.get(0))
        .expect("the row should exist");
    assert_eq!(l7, None);
}

#[test]
fn pruning_drops_history_outside_the_retention_window() {
    let (_dir, store) = store();
    let now = at(100_000);
    store
        .upsert_flows(&[
            pending(flow(1, 443, 100_000 - 3_600), None),
            pending(flow(2, 444, 100_000 - 90_000), None),
        ])
        .expect("insert should succeed");

    let policy = RetentionPolicy {
        hours: 2,
        max_rows: DEFAULT_MAX_ROWS,
    };
    let removed = store
        .prune_flows(policy, now)
        .expect("prune should succeed");

    assert_eq!(removed, 1);
    assert_eq!(
        store
            .flow_history_summary()
            .expect("summary should load")
            .stored_flows,
        1
    );
}

#[test]
fn pruning_enforces_the_row_cap_newest_first() {
    let (_dir, store) = store();
    let rows: Vec<PersistableFlow> = (0..10_u16)
        .map(|index| {
            pending(
                flow(
                    u64::from(index) + 1,
                    1_000 + index,
                    100_000 + i64::from(index),
                ),
                None,
            )
        })
        .collect();
    store.upsert_flows(&rows).expect("insert should succeed");

    let policy = RetentionPolicy {
        hours: DEFAULT_RETENTION_HOURS,
        max_rows: 4,
    };
    let removed = store
        .prune_flows(policy, at(100_010))
        .expect("prune should succeed");

    assert_eq!(removed, 6);
    let summary = store.flow_history_summary().expect("summary should load");
    assert_eq!(summary.stored_flows, 4);
    // The four survivors are the most recent ones.
    assert_eq!(
        summary.newest_seen.as_deref(),
        Some("1970-01-02T03:46:49.000Z")
    );
    assert_eq!(
        summary.oldest_seen.as_deref(),
        Some("1970-01-02T03:46:46.000Z")
    );
}

#[test]
fn an_empty_batch_touches_nothing() {
    let (_dir, store) = store();
    assert!(store
        .upsert_flows(&[])
        .expect("an empty batch should succeed")
        .is_empty());
    assert_eq!(
        store
            .flow_history_summary()
            .expect("summary should load")
            .stored_flows,
        0
    );
}

#[test]
fn the_retention_policy_falls_back_to_defaults() {
    let (_dir, store) = store();
    assert_eq!(
        store.retention_policy(),
        RetentionPolicy {
            hours: DEFAULT_RETENTION_HOURS,
            max_rows: DEFAULT_MAX_ROWS,
        }
    );

    store
        .set_setting(SETTING_RETENTION_HOURS, "6")
        .expect("setting should save");
    store
        .set_setting(SETTING_MAX_ROWS, "not a number")
        .expect("setting should save");

    let policy = store.retention_policy();
    assert_eq!(policy.hours, 6);
    assert_eq!(policy.max_rows, DEFAULT_MAX_ROWS);
}
