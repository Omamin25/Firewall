//! PID hints published by the enforcement engine's SOCKET-layer listener.
//!
//! Windows lists a socket in `GetExtendedTcpTable` only after it is established, so the
//! first packet of a connection routinely arrives before the table knows who owns it.
//! WinDivert's socket events fire at `bind`/`connect` time, which closes that gap without
//! a syscall on the packet path (`library_docs.md` §4).

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use crate::model::{FlowKey, TransportProtocol};

/// How long a hint is trusted after its last sighting. Long enough to cover a flow's
/// lifetime, short enough that a recycled port cannot inherit the wrong owner for long —
/// and a close event retires the hint immediately anyway.
const HINT_TTL: Duration = Duration::from_secs(120);
/// Upper bound on remembered endpoints, so a connection-heavy host cannot grow the map
/// without limit.
const MAX_HINTS: usize = 8_192;

type LocalKey = (TransportProtocol, IpAddr, u16);

struct Hint {
    pid: u32,
    seen: Instant,
}

/// Shared, lock-guarded index of local endpoint → owning PID. Written by the enforcement
/// engine, read by [`super::AttributionResolver`].
#[derive(Default)]
pub struct SocketHints {
    inner: Mutex<Index>,
}

#[derive(Default)]
struct Index {
    local: HashMap<LocalKey, Hint>,
    /// Sockets bound to a wildcard address, matched on port alone as a last resort.
    wildcard: HashMap<(TransportProtocol, u16), Hint>,
}

impl SocketHints {
    /// Records (or refreshes) the owner of a local endpoint.
    pub fn record(&self, protocol: TransportProtocol, local_ip: IpAddr, local_port: u16, pid: u32) {
        // PID 0 is Windows' "no owning process"; indexing it would blame the Idle process.
        if pid == 0 || local_port == 0 {
            return;
        }
        let mut index = self.lock();
        index.prune_if_full();
        let hint = Hint {
            pid,
            seen: Instant::now(),
        };
        if local_ip.is_unspecified() {
            index.wildcard.insert((protocol, local_port), hint);
        } else {
            index.local.insert((protocol, local_ip, local_port), hint);
        }
    }

    /// Retires an endpoint on socket close, before Windows can hand the port to another
    /// process.
    pub fn forget(&self, protocol: TransportProtocol, local_ip: IpAddr, local_port: u16) {
        let mut index = self.lock();
        if local_ip.is_unspecified() {
            index.wildcard.remove(&(protocol, local_port));
        } else {
            index.local.remove(&(protocol, local_ip, local_port));
        }
    }

    /// Owner of the flow's local endpoint, most specific match first. Expired hints are
    /// ignored rather than removed: pruning happens on write, off the read path.
    pub fn lookup(&self, key: &FlowKey) -> Option<u32> {
        let index = self.lock();
        index
            .fresh(
                index
                    .local
                    .get(&(key.protocol, key.local_ip, key.local_port)),
            )
            .or_else(|| index.fresh(index.wildcard.get(&(key.protocol, key.local_port))))
    }

    pub fn len(&self) -> usize {
        let index = self.lock();
        index.local.len() + index.wildcard.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A poisoned index only means one update panicked; recovering keeps enforcement
    /// running rather than taking the engine down.
    fn lock(&self) -> MutexGuard<'_, Index> {
        self.inner.lock().unwrap_or_else(|error| {
            tracing::warn!("socket hint index lock was poisoned; recovering");
            error.into_inner()
        })
    }
}

impl Index {
    fn fresh(&self, hint: Option<&Hint>) -> Option<u32> {
        hint.filter(|hint| hint.seen.elapsed() < HINT_TTL)
            .map(|hint| hint.pid)
    }

    /// Drops expired entries once the map is full; clears it outright if that was not
    /// enough, since a wrong-but-bounded cache beats an unbounded one.
    fn prune_if_full(&mut self) {
        if self.local.len() + self.wildcard.len() < MAX_HINTS {
            return;
        }
        self.local.retain(|_, hint| hint.seen.elapsed() < HINT_TTL);
        self.wildcard
            .retain(|_, hint| hint.seen.elapsed() < HINT_TTL);
        if self.local.len() + self.wildcard.len() >= MAX_HINTS {
            tracing::warn!("socket hint index is full; clearing it");
            self.local.clear();
            self.wildcard.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    fn key(local_ip: IpAddr, local_port: u16) -> FlowKey {
        FlowKey {
            protocol: TransportProtocol::Tcp,
            local_ip,
            local_port,
            remote_ip: IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            remote_port: 443,
        }
    }

    fn local() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))
    }

    #[test]
    fn resolves_a_recorded_endpoint() {
        let hints = SocketHints::default();
        hints.record(TransportProtocol::Tcp, local(), 51_000, 4_242);
        assert_eq!(hints.lookup(&key(local(), 51_000)), Some(4_242));
        assert_eq!(hints.lookup(&key(local(), 51_001)), None);
    }

    #[test]
    fn falls_back_to_a_wildcard_bind() {
        let hints = SocketHints::default();
        hints.record(
            TransportProtocol::Tcp,
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            5_353,
            77,
        );
        assert_eq!(hints.lookup(&key(local(), 5_353)), Some(77));
    }

    #[test]
    fn a_concrete_endpoint_beats_a_wildcard() {
        let hints = SocketHints::default();
        hints.record(
            TransportProtocol::Tcp,
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            8_080,
            1,
        );
        hints.record(TransportProtocol::Tcp, local(), 8_080, 2);
        assert_eq!(hints.lookup(&key(local(), 8_080)), Some(2));
    }

    #[test]
    fn close_retires_the_hint() {
        let hints = SocketHints::default();
        hints.record(TransportProtocol::Tcp, local(), 51_000, 4_242);
        hints.forget(TransportProtocol::Tcp, local(), 51_000);
        assert_eq!(hints.lookup(&key(local(), 51_000)), None);
        assert!(hints.is_empty());
    }

    #[test]
    fn ignores_unowned_and_portless_events() {
        let hints = SocketHints::default();
        hints.record(TransportProtocol::Tcp, local(), 51_000, 0);
        hints.record(TransportProtocol::Tcp, local(), 0, 4_242);
        assert!(hints.is_empty());
    }

    #[test]
    fn protocols_do_not_share_an_endpoint() {
        let hints = SocketHints::default();
        hints.record(TransportProtocol::Udp, local(), 5_000, 9);
        assert_eq!(hints.lookup(&key(local(), 5_000)), None);
    }
}
