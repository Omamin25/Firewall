pub mod frame;
#[cfg(windows)]
mod windows;

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender, TrySendError};
use thiserror::Error;

pub use frame::{CaptureInterface, RawFrame};

/// Frames buffered between the capture threads and the decoder. Sized for ~1s of
/// heavy traffic; beyond that the oldest frames are dropped and counted so the UI
/// can surface the loss instead of the capture thread blocking.
const CHANNEL_CAPACITY: usize = 8192;
/// Full Ethernet frame plus headroom — we want complete L7 payloads for SNI/DNS.
const SNAPLEN: i32 = 65_535;
/// Read timeout in ms; bounds how long a thread waits before checking its stop flag.
const READ_TIMEOUT_MS: i32 = 200;
/// `pcap_findalldevs` can block when the Npcap driver is unreachable or the process
/// is not elevated, so enumeration runs on a worker thread with this deadline.
const ENUMERATE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Npcap is unavailable or the driver could not be reached: {0}")]
    Backend(String),
    #[error("interface enumeration timed out after {}s — is Npcap installed and is Cognton running as Administrator?", ENUMERATE_TIMEOUT.as_secs())]
    EnumerateTimeout,
    #[error("no capturable network interface was found")]
    NoInterfaces,
    #[error("interface '{0}' was not found")]
    UnknownInterface(String),
    #[error("capture is already running")]
    AlreadyRunning,
    #[error("capture is not running")]
    NotRunning,
}

/// Platform capture backend. Windows uses Npcap via the `pcap` crate; the trait keeps
/// the engine free of platform detail (`architecture.md` §5 rule 3).
pub trait CaptureBackend: Send + Sync + 'static {
    fn list_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError>;

    /// Opens `interface` and pushes frames into `sink` until `stop` is set.
    /// Must never panic: errors are reported through the returned `Result` and
    /// transient read errors are logged and retried.
    fn capture(
        &self,
        interface: &CaptureInterface,
        sink: FrameSink,
        stop: Arc<AtomicBool>,
    ) -> Result<(), CaptureError>;
}

/// Bounded, drop-oldest sender shared by every capture thread.
#[derive(Clone)]
pub struct FrameSink {
    sender: Sender<RawFrame>,
    dropped: Arc<AtomicU64>,
}

impl FrameSink {
    /// Never blocks the capture thread. When the channel is full the frame is dropped
    /// and counted — back-pressure must not stall the kernel's ring buffer.
    pub fn push(&self, frame: RawFrame) {
        match self.sender.try_send(frame) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

/// Owns the capture threads and the frame channel.
pub struct CaptureEngine {
    backend: Arc<dyn CaptureBackend>,
    receiver: Receiver<RawFrame>,
    sink: FrameSink,
    dropped: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    active: std::sync::Mutex<Vec<CaptureInterface>>,
}

impl CaptureEngine {
    pub fn new() -> Self {
        Self::with_backend(default_backend())
    }

    pub fn with_backend(backend: Arc<dyn CaptureBackend>) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(CHANNEL_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        Self {
            backend,
            receiver,
            sink: FrameSink {
                sender,
                dropped: Arc::clone(&dropped),
            },
            dropped,
            stop: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            active: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Enumerates interfaces on a worker thread so a wedged driver cannot hang the
    /// caller (a Tauri command) indefinitely.
    pub fn list_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError> {
        let backend = Arc::clone(&self.backend);
        let (tx, rx) = crossbeam_channel::bounded(1);
        thread::Builder::new()
            .name("cognton-enumerate".to_owned())
            .spawn(move || {
                let _ = tx.send(backend.list_interfaces());
            })
            .map_err(|error| CaptureError::Backend(error.to_string()))?;
        rx.recv_timeout(ENUMERATE_TIMEOUT)
            .map_err(|_| CaptureError::EnumerateTimeout)?
    }

    /// Receiver the decoder pipeline drains. Cloneable; frames are distributed to
    /// whichever consumer reads first, so keep a single decoder loop.
    pub fn frames(&self) -> Receiver<RawFrame> {
        self.receiver.clone()
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn active_interfaces(&self) -> Vec<CaptureInterface> {
        self.active
            .lock()
            .map(|active| active.clone())
            .unwrap_or_default()
    }

    /// Starts one thread per interface. `requested` selects interfaces by id; when it
    /// is empty every recommended adapter is opened.
    pub fn start(&self, requested: &[String]) -> Result<Vec<CaptureInterface>, CaptureError> {
        if self.running.load(Ordering::Relaxed) {
            return Err(CaptureError::AlreadyRunning);
        }
        let available = self.list_interfaces()?;
        let selected = select_interfaces(&available, requested)?;

        self.stop.store(false, Ordering::Relaxed);
        self.running.store(true, Ordering::Relaxed);

        for interface in &selected {
            let backend = Arc::clone(&self.backend);
            let sink = self.sink.clone();
            let stop = Arc::clone(&self.stop);
            let running = Arc::clone(&self.running);
            let interface = interface.clone();
            let label = interface.name.clone();
            let thread_name = format!("cognton-capture-{}", short_id(&interface.id));
            let spawned = thread::Builder::new().name(thread_name).spawn(move || {
                let name = interface.name.clone();
                // A capture thread must never panic the app; report and exit cleanly.
                match backend.capture(&interface, sink, stop) {
                    Ok(()) => tracing::info!(interface = %name, "capture thread stopped"),
                    Err(error) => {
                        tracing::error!(interface = %name, %error, "capture thread failed");
                        running.store(false, Ordering::Relaxed);
                    }
                }
            });
            if let Err(error) = spawned {
                tracing::error!(interface = %label, %error, "could not spawn capture thread");
            }
        }

        if let Ok(mut active) = self.active.lock() {
            *active = selected.clone();
        }
        Ok(selected)
    }

    pub fn stop(&self) -> Result<(), CaptureError> {
        if !self.running.swap(false, Ordering::Relaxed) {
            return Err(CaptureError::NotRunning);
        }
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut active) = self.active.lock() {
            active.clear();
        }
        Ok(())
    }
}

impl Default for CaptureEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Explicit ids win; otherwise fall back to the recommended set, then to anything up.
fn select_interfaces(
    available: &[CaptureInterface],
    requested: &[String],
) -> Result<Vec<CaptureInterface>, CaptureError> {
    if !requested.is_empty() {
        return requested
            .iter()
            .map(|id| {
                available
                    .iter()
                    .find(|interface| &interface.id == id)
                    .cloned()
                    .ok_or_else(|| CaptureError::UnknownInterface(id.clone()))
            })
            .collect();
    }
    let recommended: Vec<_> = available
        .iter()
        .filter(|interface| interface.is_recommended)
        .cloned()
        .collect();
    if !recommended.is_empty() {
        return Ok(recommended);
    }
    let any_up: Vec<_> = available
        .iter()
        .filter(|interface| interface.is_up && !interface.is_loopback)
        .cloned()
        .collect();
    if any_up.is_empty() {
        Err(CaptureError::NoInterfaces)
    } else {
        Ok(any_up)
    }
}

/// Thread names are capped by the OS; keep the tail of the device GUID.
fn short_id(id: &str) -> String {
    id.chars().rev().take(8).collect::<String>()
}

#[cfg(windows)]
fn default_backend() -> Arc<dyn CaptureBackend> {
    Arc::new(windows::NpcapBackend::new(SNAPLEN, READ_TIMEOUT_MS))
}

#[cfg(not(windows))]
fn default_backend() -> Arc<dyn CaptureBackend> {
    Arc::new(UnsupportedBackend)
}

/// Cognton targets Windows; this keeps `cargo check` honest on other hosts.
#[cfg(not(windows))]
struct UnsupportedBackend;

#[cfg(not(windows))]
impl CaptureBackend for UnsupportedBackend {
    fn list_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError> {
        Err(CaptureError::Backend(
            "packet capture is only implemented for Windows".to_owned(),
        ))
    }

    fn capture(
        &self,
        _interface: &CaptureInterface,
        _sink: FrameSink,
        _stop: Arc<AtomicBool>,
    ) -> Result<(), CaptureError> {
        Err(CaptureError::Backend(
            "packet capture is only implemented for Windows".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    fn interface(id: &str, recommended: bool, loopback: bool) -> CaptureInterface {
        CaptureInterface {
            id: id.to_owned(),
            name: id.to_owned(),
            addresses: vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))],
            is_loopback: loopback,
            is_up: true,
            is_wireless: false,
            is_recommended: recommended,
        }
    }

    #[test]
    fn prefers_recommended_interfaces_when_none_requested() {
        let available = vec![
            interface("loopback", false, true),
            interface("wifi", true, false),
        ];
        let selected = select_interfaces(&available, &[]).expect("selection should succeed");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "wifi");
    }

    #[test]
    fn honours_explicit_selection() {
        let available = vec![
            interface("wifi", true, false),
            interface("eth", false, false),
        ];
        let selected = select_interfaces(&available, &["eth".to_owned()])
            .expect("explicit selection should succeed");
        assert_eq!(selected[0].id, "eth");
    }

    #[test]
    fn rejects_unknown_interface_id() {
        let available = vec![interface("wifi", true, false)];
        let error = select_interfaces(&available, &["nope".to_owned()])
            .expect_err("unknown id should fail");
        assert!(matches!(error, CaptureError::UnknownInterface(id) if id == "nope"));
    }

    #[test]
    fn falls_back_to_any_up_interface() {
        let available = vec![interface("eth", false, false)];
        let selected = select_interfaces(&available, &[]).expect("fallback should succeed");
        assert_eq!(selected[0].id, "eth");
    }

    #[test]
    fn sink_counts_dropped_frames_instead_of_blocking() {
        let engine = CaptureEngine::with_backend(Arc::new(NullBackend));
        let sink = engine.sink.clone();
        for _ in 0..CHANNEL_CAPACITY + 4 {
            sink.push(RawFrame {
                interface: Arc::from("test"),
                timestamp: chrono::Utc::now(),
                data: Vec::new(),
                original_len: 0,
            });
        }
        assert_eq!(engine.dropped_frames(), 4);
    }

    struct NullBackend;

    impl CaptureBackend for NullBackend {
        fn list_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError> {
            Ok(Vec::new())
        }

        fn capture(
            &self,
            _interface: &CaptureInterface,
            _sink: FrameSink,
            _stop: Arc<AtomicBool>,
        ) -> Result<(), CaptureError> {
            Ok(())
        }
    }
}
