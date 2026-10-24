//! Manual enforcement smoke check — not part of the automated suite because it loads the
//! WinDivert driver and interferes with live traffic.
//!
//! Run **elevated**:
//!   cargo run --example enforcement_smoke -- [seconds] [remote ip or CIDR] [remote port]
//!
//! With no target it opens the SOCKET-layer listener only and prints how many endpoints it
//! learns — the attribution half of Phase 4. With a target it also creates a temporary block
//! rule, so you can watch a real connection stall and then recover when the rule is toggled
//! off. This is the check behind the Phase 4 exit criteria; nothing is written to the real
//! application database.
//!
//! Example: block HTTPS to one host for 20 seconds, then restore it.
//!   cargo run --example enforcement_smoke -- 20 93.184.216.34 443

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use cognton_lib::{
    attribution::AttributionResolver,
    enforcement::{self, EnforcementEngine},
    flow::FlowTracker,
    model::{RuleAction, RuleDirection, RuleDraft, RuleProtocol},
    rules::RuleSet,
    store::Store,
};

fn main() {
    let mut args = std::env::args().skip(1);
    let seconds: u64 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(15);
    let remote_ip = args.next();
    let remote_port: Option<u16> = args.next().and_then(|value| value.parse().ok());

    let directory = match tempfile::tempdir() {
        Ok(directory) => directory,
        Err(error) => {
            eprintln!("could not create a scratch directory: {error}");
            return;
        }
    };
    let store = match Store::open(directory.path()) {
        Ok(store) => Arc::new(store),
        Err(error) => {
            eprintln!("could not open the scratch database: {error}");
            return;
        }
    };

    let resolver = Arc::new(AttributionResolver::new());
    let tracker = Arc::new(FlowTracker::new(Arc::clone(&resolver)));
    let rules = Arc::new(RuleSet::default());
    let engine = Arc::new(EnforcementEngine::new(
        enforcement::default_backend(),
        Arc::clone(&rules),
        resolver,
        tracker,
        Arc::clone(&store),
    ));

    let mut rule_id = None;
    if remote_ip.is_some() || remote_port.is_some() {
        let draft = RuleDraft {
            name: "enforcement smoke".to_owned(),
            action: RuleAction::Block,
            direction: RuleDirection::Both,
            protocol: RuleProtocol::Any,
            remote_ip: remote_ip.clone(),
            remote_port,
            local_port: None,
            process_path: None,
            user_sid: None,
            enabled: true,
            priority: 10,
        };
        match store.create_rule(&draft, Utc::now()) {
            Ok(rule) => {
                rule_id = Some(rule.id);
                let rejected = rules.load(&[rule]);
                for error in &rejected {
                    eprintln!("rule rejected: {error}");
                }
                if !rejected.is_empty() {
                    return;
                }
                println!(
                    "blocking {} port {}\n",
                    remote_ip.as_deref().unwrap_or("any address"),
                    remote_port
                        .map(|port| port.to_string())
                        .unwrap_or_else(|| "any".to_owned()),
                );
            }
            Err(error) => {
                eprintln!("could not save the temporary rule: {error}");
                return;
            }
        }
    } else {
        println!("no target given: observing socket events only\n");
    }

    engine.start();

    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(1));
        let status = engine.status();
        println!(
            "available={:<5} enforcing={:<5} observing={:<5} endpoints={:<5} socket events={:<7} blocked packets={}",
            status.available,
            status.enforcing,
            status.observing,
            status.tracked_endpoints,
            status.socket_events,
            status.blocked_packets,
        );
        if let Some(error) = status.error {
            eprintln!("\nenforcement error: {error}");
            break;
        }
    }

    // Toggling the rule off is the other half of the exit criterion: traffic must recover.
    if let Some(id) = rule_id {
        if let Err(error) = store.set_rule_enabled(id, false, Utc::now()) {
            eprintln!("could not disable the temporary rule: {error}");
        }
        rules.load(&store.list_rules().unwrap_or_default());
        engine.reconcile();
        println!(
            "\nrule disabled; enforcing={} (traffic should recover now)",
            engine.status().enforcing
        );
        thread::sleep(Duration::from_secs(2));
    }

    engine.stop();
    if let Some(id) = rule_id {
        match store.get_rule(id) {
            Ok(Some(rule)) => println!("rule recorded {} hit(s)", rule.hit_count),
            Ok(None) => println!("the temporary rule is gone"),
            Err(error) => eprintln!("could not read the temporary rule back: {error}"),
        }
    }
    println!("done");
}
