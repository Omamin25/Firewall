//! Active enforcement: the path that makes Cognton a firewall rather than a monitor.
//!
//! Two WinDivert handles do different jobs and are deliberately independent of the Npcap
//! visibility path (`architecture.md` §5 rule 4):
//! * the **network** layer intercepts packets, evaluates them against the hot rule set,
//!   and drops the matches — opened only while an enabled block rule exists, so the
//!   default allow-and-observe posture costs nothing;
//! * the **SOCKET** layer sniffs socket lifecycle events for authoritative PID ↔ 5-tuple
//!   mapping, which feeds `attribution` (`library_docs.md` §4).

pub mod engine;
pub mod meta;
#[cfg(windows)]
mod windows;

use std::sync::{atomic::AtomicBool, Arc};

use thiserror::Error;

pub use engine::{EnforcementEngine, EnforcementStatus};
pub use meta::PacketMeta;

use crate::model::TransportProtocol;

#[derive(Debug, Error)]
pub enum EnforcementError {
    #[error("the WinDivert driver could not be opened ({0}). Install WinDivert 2.x and run Cognton as Administrator.")]
    Open(String),
    #[error("the WinDivert {layer} handle failed: {message}")]
    Handle {
        layer: &'static str,
        message: String,
    },
    #[error("packet enforcement is only implemented for Windows")]
    Unsupported,
}

/// What the engine decided about one intercepted packet. `Reinject` is the default: a
/// packet the rules do not name must reach the stack unchanged.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Reinject,
    Drop,
}

/// Socket lifecycle transitions we care about. Bind and connect publish an owner; close
/// retires it before Windows can hand the port to a different process.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SocketEventKind {
    Opened,
    Closed,
}

/// One SOCKET-layer event, reduced to the facts attribution needs.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SocketEvent {
    pub kind: SocketEventKind,
    pub pid: u32,
    pub protocol: TransportProtocol,
    pub local_ip: std::net::IpAddr,
    pub local_port: u16,
}

/// Callback invoked for every intercepted packet. Must be cheap: it runs between the
/// driver handing us a packet and us handing it back.
pub type Decider = Arc<dyn Fn(&PacketMeta) -> Decision + Send + Sync>;
/// Callback invoked for every socket event.
pub type Observer = Arc<dyn Fn(SocketEvent) + Send + Sync>;
/// Called by the backend once its handle is open, so the engine can report the driver as
/// available before the first packet arrives.
pub type Ready = Arc<dyn Fn() + Send + Sync>;

/// Platform enforcement backend. The loops live behind the trait so every WinDivert type
/// stays inside `enforcement/windows.rs` (`architecture.md` §5 rule 3) and the engine can
/// be driven by a fake in tests.
pub trait EnforcementBackend: Send + Sync + 'static {
    /// Intercepts packets until `stop` is set, asking `decide` about each one. Returning
    /// an error must leave traffic flowing: the handle closes and WinDivert reinjects
    /// whatever it had queued.
    fn run_network(
        &self,
        decide: Decider,
        ready: Ready,
        stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError>;

    /// Sniffs socket events until `stop` is set. Read-only; it never blocks a connection.
    fn run_socket(
        &self,
        observe: Observer,
        ready: Ready,
        stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError>;
}

#[cfg(windows)]
pub fn default_backend() -> Arc<dyn EnforcementBackend> {
    Arc::new(windows::WinDivertBackend)
}

#[cfg(not(windows))]
pub fn default_backend() -> Arc<dyn EnforcementBackend> {
    Arc::new(UnsupportedBackend)
}

/// Cognton targets Windows; this keeps `cargo check` honest on other hosts.
#[cfg(not(windows))]
struct UnsupportedBackend;

#[cfg(not(windows))]
impl EnforcementBackend for UnsupportedBackend {
    fn run_network(
        &self,
        _decide: Decider,
        _ready: Ready,
        _stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError> {
        Err(EnforcementError::Unsupported)
    }

    fn run_socket(
        &self,
        _observe: Observer,
        _ready: Ready,
        _stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError> {
        Err(EnforcementError::Unsupported)
    }
}

#[cfg(test)]
mod tests;
