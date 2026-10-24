//! Manual capture smoke check — not part of the automated test suite because it needs
//! the Npcap driver and Administrator rights.
//!
//! Run from an elevated shell:
//!   cargo run --example capture_smoke -- [seconds]
//!
//! Prints the interfaces Npcap reports, then decodes live frames for a few seconds and
//! summarises what the L7 classifiers recognised. This is the check behind the Phase 2
//! exit criteria ("browsing produces correctly-decoded packets within ~1s").

use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use cognton_lib::{
    capture::CaptureEngine,
    parse::{self, DecodeContext},
};

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(8);

    let engine = CaptureEngine::new();

    let interfaces = match engine.list_interfaces() {
        Ok(interfaces) => interfaces,
        Err(error) => {
            eprintln!("interface enumeration failed: {error}");
            return;
        }
    };

    println!("{} interface(s) reported by Npcap:", interfaces.len());
    for interface in &interfaces {
        println!(
            "  [{}] {} — up={} loopback={} wireless={} addrs={:?}",
            if interface.is_recommended { "*" } else { " " },
            interface.name,
            interface.is_up,
            interface.is_loopback,
            interface.is_wireless,
            interface.addresses
        );
    }

    let started = match engine.start(&[]) {
        Ok(started) => started,
        Err(error) => {
            eprintln!("capture failed to start: {error}");
            return;
        }
    };
    println!(
        "\ncapturing on {} interface(s) for {seconds}s…",
        started.len()
    );

    let contexts: BTreeMap<String, DecodeContext> = started
        .iter()
        .map(|interface| {
            (
                interface.id.clone(),
                DecodeContext {
                    local_macs: Vec::new(),
                    local_ips: interface.routable_addresses(),
                },
            )
        })
        .collect();

    let frames = engine.frames();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let fallback = DecodeContext::default();
    let mut total = 0_u64;
    let mut by_protocol: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut hosts: BTreeMap<String, u64> = BTreeMap::new();

    while Instant::now() < deadline {
        let Ok(frame) = frames.recv_timeout(Duration::from_millis(200)) else {
            continue;
        };
        let context = contexts.get(frame.interface.as_ref()).unwrap_or(&fallback);
        if let Some(decoded) = parse::decode(&frame.data, context) {
            total += 1;
            *by_protocol.entry(decoded.protocol.as_str()).or_default() += 1;
            if let Some(host) = decoded.remote_host {
                *hosts.entry(host).or_default() += 1;
            }
        }
    }

    let _ = engine.stop();

    println!(
        "\ndecoded {total} frame(s); dropped {}",
        engine.dropped_frames()
    );
    println!("by protocol:");
    for (protocol, count) in &by_protocol {
        println!("  {protocol:<6} {count}");
    }
    println!("resolved hosts (DNS names / TLS SNI): {}", hosts.len());
    for (host, count) in hosts.iter().take(20) {
        println!("  {host}  ({count})");
    }
}
