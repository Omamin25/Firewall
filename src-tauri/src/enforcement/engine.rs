//! Orchestration for the enforcement path.
//!
//! Lifecycle, in one place so the invariants are checkable:
//! * the SOCKET-layer listener runs for the whole app session — it only observes, and its
//!   PID hints improve attribution whether or not any rule exists;
//! * the network-layer interceptor runs **only while an enabled block rule exists**. With
//!   no block rules the driver never sees our filter, which keeps the default
//!   allow-and-observe posture free (`architecture.md` §5 rule 7);
//! * every failure fails **open**: the handle closes, WinDivert reinjects what it queued,
//!   and the error is surfaced to the UI rather than silently dropping traffic
//!   (`architecture.md` §5 rule 8).

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use super::{
    Decision, EnforcementBackend, EnforcementError, PacketMeta, SocketEvent, SocketEventKind,
};
use crate::{
    attribution::{AttributionResolver, SocketHints},
    flow::FlowTracker,
    model::Attribution,
    rules::{Candidate, RuleSet},
    store::Store,
};

/// How often accumulated rule hits are written back. Counting is in memory so the
/// database stays off the packet path (`code_standards.md` §2).
const HIT_FLUSH_INTERVAL: Duration = Duration::from_secs(2);
/// Maintenance tick; short enough that a stop request is not waited out.
const MAINTENANCE_TICK: Duration = Duration::from_millis(250);

/// What the UI needs to render the enforcement half of the engine-status pill.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnforcementStatus {
    /// True once a WinDivert handle has opened — i.e. the driver is present and Cognton
    /// is elevated enough to use it.
    pub available: bool,
    /// True while the network-layer handle is intercepting packets.
    pub enforcing: bool,
    /// True while the SOCKET-layer listener is observing.
    pub observing: bool,
    pub enabled_rules: usize,
    pub block_rules: usize,
    pub blocked_packets: u64,
    pub socket_events: u64,
    pub tracked_endpoints: usize,
    /// Actionable message when a handle could not be opened or died.
    pub error: Option<String>,
}

/// A running backend loop and the flag that stops it.
struct Worker {
    stop: Arc<AtomicBool>,
    /// Cleared by the thread itself when its loop returns, so a dead worker is
    /// distinguishable from a live one.
    live: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Worker {
    fn is_live(&self) -> bool {
        self.live.load(Ordering::Relaxed)
    }

    fn shut_down(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[derive(Default)]
struct Threads {
    socket: Option<Worker>,
    network: Option<Worker>,
    maintenance: Option<Worker>,
}

pub struct EnforcementEngine {
    backend: Arc<dyn EnforcementBackend>,
    rules: Arc<RuleSet>,
    resolver: Arc<AttributionResolver>,
    hints: Arc<SocketHints>,
    tracker: Arc<FlowTracker>,
    store: Arc<Store>,
    threads: Mutex<Threads>,
    available: AtomicBool,
    blocked_packets: AtomicU64,
    socket_events: AtomicU64,
    error: Mutex<Option<String>>,
}

impl EnforcementEngine {
    pub fn new(
        backend: Arc<dyn EnforcementBackend>,
        rules: Arc<RuleSet>,
        resolver: Arc<AttributionResolver>,
        tracker: Arc<FlowTracker>,
        store: Arc<Store>,
    ) -> Self {
        Self {
            backend,
            rules,
            hints: resolver.hints(),
            resolver,
            tracker,
            store,
            threads: Mutex::new(Threads::default()),
            available: AtomicBool::new(false),
            blocked_packets: AtomicU64::new(0),
            socket_events: AtomicU64::new(0),
            error: Mutex::new(None),
        }
    }

    /// Starts the socket listener and the hit writer, then opens a network handle if the
    /// rule set already blocks something. Never fails: a missing driver is a degraded
    /// state, not a startup failure.
    pub fn start(self: &Arc<Self>) {
        self.spawn_socket();
        self.spawn_maintenance();
        self.reconcile();
    }

    /// Brings the network handle in line with the rule set. Called after every rule
    /// mutation — this is what makes enforcement hot-reload without a restart. A handle
    /// that died from an error is retried here, so fixing the cause and toggling a rule
    /// recovers without relaunching.
    pub fn reconcile(self: &Arc<Self>) {
        let wanted = self.rules.has_block_rules();
        let dead = {
            let threads = self.lock_threads();
            threads
                .network
                .as_ref()
                .is_some_and(|worker| !worker.is_live())
        };
        if dead {
            if let Some(worker) = self.lock_threads().network.take() {
                worker.shut_down();
            }
        }

        let present = self.lock_threads().network.is_some();
        match (wanted, present) {
            (true, false) => self.spawn_network(),
            (false, true) => {
                if let Some(worker) = self.lock_threads().network.take() {
                    worker.shut_down();
                    tracing::info!("no block rules remain; enforcement handle closed");
                }
            }
            _ => {}
        }
    }

    /// Stops every worker. Traffic flows normally afterwards.
    pub fn stop(&self) {
        let workers = {
            let mut threads = self.lock_threads();
            [
                threads.network.take(),
                threads.socket.take(),
                threads.maintenance.take(),
            ]
        };
        for worker in workers.into_iter().flatten() {
            worker.shut_down();
        }
        self.flush_hits();
    }

    pub fn status(&self) -> EnforcementStatus {
        let (enforcing, observing) = {
            let threads = self.lock_threads();
            (
                threads.network.as_ref().is_some_and(Worker::is_live),
                threads.socket.as_ref().is_some_and(Worker::is_live),
            )
        };
        EnforcementStatus {
            available: self.available.load(Ordering::Relaxed),
            enforcing,
            observing,
            enabled_rules: self.rules.len(),
            block_rules: self.rules.block_rule_count(),
            blocked_packets: self.blocked_packets.load(Ordering::Relaxed),
            socket_events: self.socket_events.load(Ordering::Relaxed),
            tracked_endpoints: self.hints.len(),
            error: self.lock_error().clone(),
        }
    }

    /// Clears a stale failure so a retry — after installing the driver, say — is not
    /// masked by the message from the previous attempt.
    pub fn clear_error(&self) {
        *self.lock_error() = None;
    }

    /// The verdict for one intercepted packet. Kept allocation- and syscall-free on the
    /// common path: with no process/user rules loaded it is a handful of comparisons.
    fn decide(&self, meta: &PacketMeta) -> Decision {
        let owner = if self.rules.needs_attribution() {
            self.owner(meta)
        } else {
            Attribution::EMPTY
        };
        let verdict = self.rules.evaluate(&Candidate {
            direction: meta.direction,
            protocol: meta.protocol,
            local_port: meta.local_port,
            remote_ip: meta.remote_ip,
            remote_port: meta.remote_port,
            process_path: owner.process_path.as_deref(),
            user_sid: owner.user_sid.as_deref(),
        });

        if !verdict.is_block() {
            return Decision::Reinject;
        }
        self.blocked_packets.fetch_add(1, Ordering::Relaxed);
        if let Some(matched) = verdict.rule {
            // Logged once per flow, not once per packet: every enforcement action is on
            // the record (`architecture.md` §5 rule 7) without flooding the log.
            if self.tracker.mark_blocked(&meta.flow_key(), matched.clone()) {
                tracing::info!(
                    rule = %matched.name,
                    rule_id = matched.id,
                    remote = %meta.remote_ip,
                    remote_port = meta.remote_port,
                    protocol = meta.protocol.as_str(),
                    direction = meta.direction.as_str(),
                    "blocked flow"
                );
            }
        }
        Decision::Drop
    }

    /// Owner of the packet's local endpoint. Socket hints answer most lookups without a
    /// syscall; the resolver's own TTL cache bounds the rest.
    fn owner(&self, meta: &PacketMeta) -> Attribution {
        let key = meta.flow_key();
        match self.hints.lookup(&key) {
            Some(pid) => self.resolver.describe(pid),
            None => self.resolver.resolve(&key),
        }
    }

    fn observe(&self, event: SocketEvent) {
        self.socket_events.fetch_add(1, Ordering::Relaxed);
        match event.kind {
            SocketEventKind::Opened => {
                self.hints
                    .record(event.protocol, event.local_ip, event.local_port, event.pid)
            }
            SocketEventKind::Closed => {
                self.hints
                    .forget(event.protocol, event.local_ip, event.local_port)
            }
        }
    }

    fn spawn_network(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        let decide = {
            let engine = Arc::clone(self);
            Arc::new(move |meta: &PacketMeta| engine.decide(meta))
        };
        let worker = self.spawn("cognton-enforce", "network", move |ready, stop| {
            engine.backend.run_network(decide, ready, stop)
        });
        if let Some(worker) = worker {
            self.lock_threads().network = Some(worker);
            tracing::info!(
                rules = self.rules.block_rule_count(),
                "enforcement handle requested"
            );
        }
    }

    fn spawn_socket(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        let observe = {
            let engine = Arc::clone(self);
            Arc::new(move |event: SocketEvent| engine.observe(event))
        };
        let worker = self.spawn("cognton-sockets", "socket", move |ready, stop| {
            engine.backend.run_socket(observe, ready, stop)
        });
        if let Some(worker) = worker {
            self.lock_threads().socket = Some(worker);
        }
    }

    /// Wraps a backend loop in a thread that records its own failure instead of taking the
    /// app down (`code_standards.md` §2: no panics in engine threads).
    fn spawn<F>(self: &Arc<Self>, name: &str, layer: &'static str, run: F) -> Option<Worker>
    where
        F: FnOnce(super::Ready, Arc<AtomicBool>) -> Result<(), EnforcementError> + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let live = Arc::new(AtomicBool::new(false));
        let engine = Arc::clone(self);
        let thread_stop = Arc::clone(&stop);
        let thread_live = Arc::clone(&live);
        let ready_live = Arc::clone(&live);
        let ready_engine = Arc::clone(self);
        let ready: super::Ready = Arc::new(move || {
            ready_live.store(true, Ordering::Relaxed);
            ready_engine.available.store(true, Ordering::Relaxed);
            ready_engine.clear_error();
        });

        let spawned = thread::Builder::new().name(name.to_owned()).spawn(move || {
            let outcome = run(ready, Arc::clone(&thread_stop));
            thread_live.store(false, Ordering::Relaxed);
            match outcome {
                Ok(()) => tracing::info!(layer, "enforcement thread stopped"),
                Err(error) => {
                    tracing::error!(layer, %error, "enforcement thread failed");
                    engine.record_error(error.to_string());
                }
            }
        });
        match spawned {
            Ok(join) => Some(Worker {
                stop,
                live,
                join: Some(join),
            }),
            Err(error) => {
                tracing::error!(layer, %error, "could not spawn enforcement thread");
                self.record_error(format!("could not start the {layer} thread: {error}"));
                None
            }
        }
    }

    /// Slow tick that writes accumulated hit counts back. Separate from the intercept loop
    /// because that loop is blocked inside the driver waiting for packets.
    fn spawn_maintenance(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        let stop = Arc::new(AtomicBool::new(false));
        let live = Arc::new(AtomicBool::new(true));
        let thread_stop = Arc::clone(&stop);
        let thread_live = Arc::clone(&live);
        let spawned = thread::Builder::new()
            .name("cognton-enforce-hits".to_owned())
            .spawn(move || {
                let mut since_flush = Duration::ZERO;
                while !thread_stop.load(Ordering::Relaxed) {
                    thread::sleep(MAINTENANCE_TICK);
                    since_flush += MAINTENANCE_TICK;
                    if since_flush >= HIT_FLUSH_INTERVAL {
                        since_flush = Duration::ZERO;
                        engine.flush_hits();
                    }
                }
                thread_live.store(false, Ordering::Relaxed);
                engine.flush_hits();
            });
        match spawned {
            Ok(join) => {
                self.lock_threads().maintenance = Some(Worker {
                    stop,
                    live,
                    join: Some(join),
                });
            }
            // Losing this thread costs persisted hit counts, not enforcement, so it is a
            // warning rather than a surfaced error.
            Err(error) => tracing::warn!(%error, "could not start the hit-count writer"),
        }
    }

    fn flush_hits(&self) {
        let deltas = self.rules.take_hit_deltas();
        if deltas.is_empty() {
            return;
        }
        if let Err(error) = self.store.add_rule_hits(&deltas) {
            tracing::warn!(%error, rules = deltas.len(), "rule hit counts could not be saved");
        }
    }

    fn record_error(&self, message: String) {
        *self.lock_error() = Some(message);
    }

    fn lock_threads(&self) -> MutexGuard<'_, Threads> {
        self.threads.lock().unwrap_or_else(|error| {
            tracing::warn!("enforcement thread table lock was poisoned; recovering");
            error.into_inner()
        })
    }

    fn lock_error(&self) -> MutexGuard<'_, Option<String>> {
        self.error.lock().unwrap_or_else(|error| error.into_inner())
    }
}
