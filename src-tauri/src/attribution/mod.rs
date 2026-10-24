//! Maps observed flows onto the process and Windows user that own them.
//!
//! The OS syscalls behind this are expensive relative to a packet, so everything is
//! cached: the connection table is snapshotted at most a few times a second and
//! process/user lookups are memoised per PID (`code_standards.md` §2).

mod snapshot;
#[cfg(windows)]
mod windows;

pub mod hints;

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::{Duration, Instant},
};

use thiserror::Error;

use crate::model::{Attribution, FlowKey, TransportProtocol};
pub use hints::SocketHints;
use snapshot::Snapshot;

/// How long a connection-table snapshot is trusted before a refresh.
const TABLE_TTL: Duration = Duration::from_millis(1_000);
/// Floor on forced refreshes. A brand-new flow is often not in the table yet, so a miss
/// triggers an out-of-cycle refresh — but no more often than this.
const MIN_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
/// Process metadata lifetime. Kept short because Windows recycles PIDs; a stale entry
/// would attribute a new process's traffic to the one that just exited.
const PROCESS_TTL: Duration = Duration::from_secs(30);
/// Upper bound on memoised processes, so a fork-heavy workload cannot grow the cache
/// without limit.
const MAX_CACHED_PROCESSES: usize = 512;

#[derive(Debug, Error)]
pub enum AttributionError {
    #[error("the OS connection table could not be read: {0}")]
    ConnectionTable(String),
    #[error("process {pid} could not be inspected: {message}")]
    Process { pid: u32, message: String },
    #[error("process and user attribution is only implemented for Windows")]
    Unsupported,
}

/// One row of the OS connection table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionOwner {
    pub protocol: TransportProtocol,
    pub local_ip: IpAddr,
    pub local_port: u16,
    /// The peer, for connected TCP rows. Listening sockets and UDP endpoints have none.
    pub remote: Option<(IpAddr, u16)>,
    pub pid: u32,
}

/// Process identity as far as it could be read. Fields stay `None` for protected
/// processes, which deny `OpenProcess` even to Administrators.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProcessInfo {
    pub name: Option<String>,
    pub path: Option<String>,
    pub user_account: Option<String>,
    pub user_sid: Option<String>,
}

/// Platform attribution source. Keeps the resolver free of Win32 detail
/// (`architecture.md` §5 rule 3).
pub trait AttributionBackend: Send + Sync + 'static {
    fn connections(&self) -> Result<Vec<ConnectionOwner>, AttributionError>;
    fn process(&self, pid: u32) -> Result<ProcessInfo, AttributionError>;
}

/// Cache tuning. Exposed so tests can pin the TTLs instead of sleeping.
#[derive(Clone, Copy, Debug)]
pub struct ResolverConfig {
    pub table_ttl: Duration,
    pub min_refresh_interval: Duration,
    pub process_ttl: Duration,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            table_ttl: TABLE_TTL,
            min_refresh_interval: MIN_REFRESH_INTERVAL,
            process_ttl: PROCESS_TTL,
        }
    }
}

pub struct AttributionResolver {
    backend: Arc<dyn AttributionBackend>,
    config: ResolverConfig,
    table: Mutex<Snapshot>,
    processes: Mutex<HashMap<u32, CachedProcess>>,
    /// Endpoint owners published by the enforcement engine's SOCKET-layer listener.
    /// Empty until that engine starts, in which case only the OS table is consulted.
    hints: Arc<SocketHints>,
    /// Set while the connection table is unreadable, so the failure is logged on
    /// transition rather than once per packet.
    degraded: AtomicBool,
}

struct CachedProcess {
    stored: Instant,
    info: ProcessInfo,
}

impl AttributionResolver {
    pub fn new() -> Self {
        Self::with_backend(default_backend())
    }

    pub fn with_backend(backend: Arc<dyn AttributionBackend>) -> Self {
        Self::with_config(backend, ResolverConfig::default())
    }

    pub fn with_config(backend: Arc<dyn AttributionBackend>, config: ResolverConfig) -> Self {
        Self {
            backend,
            config,
            table: Mutex::new(Snapshot::default()),
            processes: Mutex::new(HashMap::new()),
            hints: Arc::new(SocketHints::default()),
            degraded: AtomicBool::new(false),
        }
    }

    /// The hint index this resolver reads. Handed to the enforcement engine so its
    /// SOCKET-layer listener writes into the same instance.
    pub fn hints(&self) -> Arc<SocketHints> {
        Arc::clone(&self.hints)
    }

    /// Best-effort owner of `key`. Returns an empty [`Attribution`] rather than an error
    /// when nothing can be resolved — an unattributed flow is still worth showing.
    pub fn resolve(&self, key: &FlowKey) -> Attribution {
        // ICMP and IGMP have no socket, so neither source can own them.
        if !key.protocol.has_ports() {
            return Attribution::default();
        }

        self.refresh_if_older_than(self.config.table_ttl);
        // The OS table is authoritative and already cached, so it is tried first; socket
        // hints then cover the window before a new connection appears in it.
        let mut pid = self.lookup(key).or_else(|| self.hints.lookup(key));
        if pid.is_none() {
            // One extra, rate-limited refresh for a flow the kernel has caught up on.
            self.refresh_if_older_than(self.config.min_refresh_interval);
            pid = self.lookup(key);
        }

        match pid {
            Some(pid) => self.describe(pid),
            None => Attribution::default(),
        }
    }

    /// Process and user identity for a PID, from the memoised cache. Public so the
    /// enforcement path can attribute a packet it already has a PID for.
    pub fn describe(&self, pid: u32) -> Attribution {
        let info = self.process_info(pid);
        Attribution {
            pid: Some(pid),
            process_name: info.name,
            process_path: info.path,
            user_account: info.user_account,
            user_sid: info.user_sid,
        }
    }

    fn lookup(&self, key: &FlowKey) -> Option<u32> {
        self.lock_table().lookup(key)
    }

    fn process_info(&self, pid: u32) -> ProcessInfo {
        if let Some(info) = self.cached_process(pid) {
            return info;
        }
        // A failure is cached too: protected processes fail every time, and retrying
        // per packet would put an `OpenProcess` on the decode hot path.
        let info = self.backend.process(pid).unwrap_or_else(|error| {
            tracing::debug!(pid, %error, "process metadata unavailable");
            ProcessInfo::default()
        });
        self.store_process(pid, info.clone());
        info
    }

    fn cached_process(&self, pid: u32) -> Option<ProcessInfo> {
        let processes = self.lock_processes();
        processes
            .get(&pid)
            .filter(|cached| cached.stored.elapsed() < self.config.process_ttl)
            .map(|cached| cached.info.clone())
    }

    fn store_process(&self, pid: u32, info: ProcessInfo) {
        let mut processes = self.lock_processes();
        if processes.len() >= MAX_CACHED_PROCESSES {
            let ttl = self.config.process_ttl;
            processes.retain(|_, cached| cached.stored.elapsed() < ttl);
            if processes.len() >= MAX_CACHED_PROCESSES {
                processes.clear();
            }
        }
        processes.insert(
            pid,
            CachedProcess {
                stored: Instant::now(),
                info,
            },
        );
    }

    /// Rebuilds the snapshot when it is older than `max_age`. The backend call happens
    /// outside the lock so a slow syscall never blocks a reader.
    fn refresh_if_older_than(&self, max_age: Duration) {
        if self.lock_table().age() < max_age {
            return;
        }
        match self.backend.connections() {
            Ok(rows) => {
                if self.degraded.swap(false, Ordering::Relaxed) {
                    tracing::info!("connection table is readable again");
                }
                *self.lock_table() = Snapshot::build(rows);
            }
            Err(error) => {
                if !self.degraded.swap(true, Ordering::Relaxed) {
                    tracing::warn!(%error, "process attribution is unavailable");
                }
                // Mark the attempt so a failing backend is not retried per packet.
                self.lock_table().mark_attempted();
            }
        }
    }

    /// A poisoned cache only means one lookup panicked; recovering keeps the decode
    /// thread alive rather than taking down capture.
    fn lock_table(&self) -> MutexGuard<'_, Snapshot> {
        self.table.lock().unwrap_or_else(|error| {
            tracing::warn!("attribution table lock was poisoned; recovering");
            error.into_inner()
        })
    }

    fn lock_processes(&self) -> MutexGuard<'_, HashMap<u32, CachedProcess>> {
        self.processes.lock().unwrap_or_else(|error| {
            tracing::warn!("process cache lock was poisoned; recovering");
            error.into_inner()
        })
    }
}

impl Default for AttributionResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
pub fn default_backend() -> Arc<dyn AttributionBackend> {
    Arc::new(windows::WindowsAttribution::default())
}

#[cfg(not(windows))]
pub fn default_backend() -> Arc<dyn AttributionBackend> {
    Arc::new(UnsupportedAttribution)
}

/// Cognton targets Windows; this keeps `cargo check` honest on other hosts.
#[cfg(not(windows))]
struct UnsupportedAttribution;

#[cfg(not(windows))]
impl AttributionBackend for UnsupportedAttribution {
    fn connections(&self) -> Result<Vec<ConnectionOwner>, AttributionError> {
        Err(AttributionError::Unsupported)
    }

    fn process(&self, _pid: u32) -> Result<ProcessInfo, AttributionError> {
        Err(AttributionError::Unsupported)
    }
}

#[cfg(test)]
mod tests;
