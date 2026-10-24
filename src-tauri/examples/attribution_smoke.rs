//! Manual attribution smoke check — not part of the automated test suite because it
//! reads live OS state.
//!
//! Run:
//!   cargo run --example attribution_smoke -- [max rows]
//!
//! Prints the OS connection table with the owning process and Windows user for each
//! row, then re-resolves the same rows through the cache to show the resolver is not
//! going back to the kernel per lookup. This is the check behind the Phase 3 exit
//! criterion "every connection shows correct process and user". Unlike capture, this
//! path does not need Administrator rights — though without elevation some processes
//! deny `OpenProcess` and show as unknown.

use std::{sync::Arc, time::Instant};

use cognton_lib::{attribution::AttributionResolver, model::FlowKey};

fn main() {
    let limit: usize = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(25);

    let backend = cognton_lib::attribution::default_backend();
    let rows = match backend.connections() {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("could not read the OS connection table: {error}");
            return;
        }
    };
    println!("{} connection table row(s)\n", rows.len());

    let resolver = AttributionResolver::with_backend(Arc::clone(&backend));
    let mut resolved = 0_usize;
    let mut named = 0_usize;

    for row in rows.iter().take(limit) {
        let key = FlowKey {
            protocol: row.protocol,
            local_ip: row.local_ip,
            local_port: row.local_port,
            remote_ip: row
                .remote
                .map(|(address, _)| address)
                .unwrap_or(row.local_ip),
            remote_port: row.remote.map(|(_, port)| port).unwrap_or(0),
        };
        let owner = resolver.resolve(&key);
        if owner.pid.is_some() {
            resolved += 1;
        }
        if owner.process_name.is_some() {
            named += 1;
        }
        println!(
            "  {:<4} {:>21} -> {:<21} pid={:<7} {:<28} {}",
            row.protocol.as_str(),
            format!("{}:{}", row.local_ip, row.local_port),
            row.remote
                .map(|(address, port)| format!("{address}:{port}"))
                .unwrap_or_else(|| "-".to_owned()),
            owner.pid.map(|pid| pid.to_string()).unwrap_or_default(),
            owner.process_name.unwrap_or_else(|| "<unknown>".to_owned()),
            owner.user_account.unwrap_or_else(|| "<unknown>".to_owned()),
        );
    }

    println!("\n{resolved} of {limit} shown rows resolved to a PID, {named} to a process name");

    // A second pass must be served entirely from the caches.
    let started = Instant::now();
    for row in rows.iter().take(limit) {
        let key = FlowKey {
            protocol: row.protocol,
            local_ip: row.local_ip,
            local_port: row.local_port,
            remote_ip: row
                .remote
                .map(|(address, _)| address)
                .unwrap_or(row.local_ip),
            remote_port: row.remote.map(|(_, port)| port).unwrap_or(0),
        };
        let _ = resolver.resolve(&key);
    }
    println!(
        "cached re-resolution of {limit} rows took {:?}",
        started.elapsed()
    );
}
