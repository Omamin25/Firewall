//! Indexed view of one connection-table read.
//!
//! Three maps are tried most-specific first: the full five-tuple, then the local
//! endpoint, then the port alone for wildcard-bound sockets. Anything looser would
//! attribute traffic to the wrong process more often than it would help.

use std::{
    collections::HashMap,
    net::IpAddr,
    time::{Duration, Instant},
};

use super::ConnectionOwner;
use crate::model::{FlowKey, TransportProtocol};

type ExactKey = (TransportProtocol, IpAddr, u16, IpAddr, u16);
type LocalKey = (TransportProtocol, IpAddr, u16);

#[derive(Default)]
pub(super) struct Snapshot {
    taken: Option<Instant>,
    exact: HashMap<ExactKey, u32>,
    local: HashMap<LocalKey, u32>,
    /// Sockets bound to a wildcard address, matched on port alone as a last resort.
    wildcard: HashMap<(TransportProtocol, u16), u32>,
}

impl Snapshot {
    pub(super) fn build(rows: Vec<ConnectionOwner>) -> Self {
        let mut snapshot = Self {
            taken: Some(Instant::now()),
            ..Self::default()
        };
        for row in rows {
            // Windows reports PID 0 for sockets with no owning process — TIME_WAIT
            // remnants, mostly. Indexing those would attribute live traffic to Idle.
            if row.pid == 0 {
                continue;
            }
            if let Some((remote_ip, remote_port)) = row.remote {
                snapshot
                    .exact
                    .entry((
                        row.protocol,
                        row.local_ip,
                        row.local_port,
                        remote_ip,
                        remote_port,
                    ))
                    .or_insert(row.pid);
            }
            // Duplicate local endpoints happen with `SO_REUSEADDR`; the first row wins.
            snapshot
                .local
                .entry((row.protocol, row.local_ip, row.local_port))
                .or_insert(row.pid);
            if row.local_ip.is_unspecified() {
                snapshot
                    .wildcard
                    .entry((row.protocol, row.local_port))
                    .or_insert(row.pid);
            }
        }
        snapshot
    }

    /// `Duration::MAX` before the first successful or attempted read, so the caller
    /// always refreshes once.
    pub(super) fn age(&self) -> Duration {
        self.taken.map(|at| at.elapsed()).unwrap_or(Duration::MAX)
    }

    /// Records a failed read so the failing backend is not retried for every packet.
    pub(super) fn mark_attempted(&mut self) {
        self.taken = Some(Instant::now());
    }

    pub(super) fn lookup(&self, key: &FlowKey) -> Option<u32> {
        let exact = (
            key.protocol,
            key.local_ip,
            key.local_port,
            key.remote_ip,
            key.remote_port,
        );
        if let Some(pid) = self.exact.get(&exact) {
            return Some(*pid);
        }
        if let Some(pid) = self
            .local
            .get(&(key.protocol, key.local_ip, key.local_port))
        {
            return Some(*pid);
        }
        // Dual-stack wildcard sockets serve both families, so the address is ignored
        // here. This is the least precise match and is only reached after the two above.
        self.wildcard.get(&(key.protocol, key.local_port)).copied()
    }
}
