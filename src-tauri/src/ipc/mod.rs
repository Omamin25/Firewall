pub mod dto;
pub mod flows;
pub mod ring;
pub mod rules;
pub mod stream;

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

use tauri::{ipc::Channel, State};

use crate::{
    attribution::AttributionResolver,
    capture::CaptureEngine,
    enforcement::{self, EnforcementEngine},
    flow::FlowTracker,
    is_elevated,
    rules::RuleSet,
    store::Store,
};
use dto::{CaptureInterfaceDto, CaptureStatusDto, PacketDetailDto, StreamMessage};
use ring::PacketRing;
use stream::{PacketPipeline, StreamHub};

/// Decoded packets retained in memory for the inspector. ~2 minutes of moderate
/// traffic; per-packet data is never persisted.
const PACKET_RING_CAPACITY: usize = 20_000;

pub struct AppState {
    pub store: Arc<Store>,
    pub startup_error: Option<String>,
    pub capture: Arc<CaptureEngine>,
    pub hub: Arc<StreamHub>,
    pub ring: Arc<PacketRing>,
    pub tracker: Arc<FlowTracker>,
    /// The enabled rules, compiled and held in memory for the enforcement path.
    pub rules: Arc<RuleSet>,
    pub enforcement: Arc<EnforcementEngine>,
    /// Packet ids are allocated app-wide so they stay unique across capture restarts.
    next_packet_id: Arc<AtomicU64>,
    pipeline: Mutex<Option<PipelineHandle>>,
}

struct PipelineHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl AppState {
    pub fn new(store: Arc<Store>, startup_error: Option<String>) -> Self {
        let resolver = Arc::new(AttributionResolver::new());
        let tracker = Arc::new(FlowTracker::new(Arc::clone(&resolver)));
        let rules = Arc::new(RuleSet::default());
        let enforcement = Arc::new(EnforcementEngine::new(
            enforcement::default_backend(),
            Arc::clone(&rules),
            resolver,
            Arc::clone(&tracker),
            Arc::clone(&store),
        ));
        Self {
            store,
            startup_error,
            capture: Arc::new(CaptureEngine::new()),
            hub: Arc::new(StreamHub::default()),
            ring: Arc::new(PacketRing::new(PACKET_RING_CAPACITY)),
            tracker,
            rules,
            enforcement,
            next_packet_id: Arc::new(AtomicU64::new(1)),
            pipeline: Mutex::new(None),
        }
    }

    /// Loads the persisted rules into the hot set and starts the enforcement engine. A
    /// failure here is reported, not fatal: the app still captures and observes.
    pub fn start_enforcement(&self) {
        match self.store.list_rules() {
            Ok(stored) => {
                for rejected in self.rules.load(&stored) {
                    tracing::warn!(%rejected, "a stored rule could not be compiled");
                }
                tracing::info!(
                    rules = self.rules.len(),
                    blocking = self.rules.block_rule_count(),
                    "rule set loaded"
                );
            }
            Err(error) => tracing::error!(%error, "stored rules could not be loaded"),
        }
        self.enforcement.start();
    }

    /// Signals the decode thread to stop and joins it. Safe to call when idle.
    fn shutdown_pipeline(&self) {
        let handle = self
            .pipeline
            .lock()
            .ok()
            .and_then(|mut pipeline| pipeline.take());
        if let Some(mut handle) = handle {
            handle.stop.store(true, Ordering::Relaxed);
            if let Some(join) = handle.join.take() {
                let _ = join.join();
            }
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    pub is_elevated: bool,
    pub database_ready: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn demo_health_check(state: State<'_, AppState>) -> AppStatusDto {
    AppStatusDto {
        is_elevated: is_elevated(),
        database_ready: state.startup_error.is_none(),
        error: state.startup_error.clone(),
    }
}

#[tauri::command]
pub fn database_smoke_test(state: State<'_, AppState>) -> Result<String, String> {
    state
        .store
        .set_setting("phase_1_smoke", "ok")
        .map_err(|error| error.to_string())?;
    state
        .store
        .get_setting("phase_1_smoke")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "setting was not persisted".to_owned())
}

/// Lists capturable adapters. Surfaces Npcap/elevation failures as an error string so
/// the UI can render an actionable state instead of an empty table.
#[tauri::command]
pub fn list_capture_interfaces(
    state: State<'_, AppState>,
) -> Result<Vec<CaptureInterfaceDto>, String> {
    state
        .capture
        .list_interfaces()
        .map(|interfaces| {
            interfaces
                .iter()
                .map(CaptureInterfaceDto::from_interface)
                .collect()
        })
        .map_err(|error| error.to_string())
}

/// Starts capture on `interfaceIds`, or on every recommended adapter when empty.
#[tauri::command]
pub fn start_capture(
    state: State<'_, AppState>,
    interface_ids: Option<Vec<String>>,
) -> Result<CaptureStatusDto, String> {
    let requested = validate_interface_ids(interface_ids.unwrap_or_default())?;

    if state.capture.is_running() {
        return Ok(capture_status(&state, None));
    }

    let interfaces = state.capture.start(&requested).map_err(|error| {
        let message = error.to_string();
        tracing::error!(%message, "capture failed to start");
        message
    })?;

    let stop = Arc::new(AtomicBool::new(false));
    let capture = Arc::clone(&state.capture);
    let dropped: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(move || capture.dropped_frames());
    let pipeline = Arc::new(PacketPipeline::new(
        Arc::clone(&state.hub),
        Arc::clone(&state.ring),
        Arc::clone(&state.tracker),
        Arc::clone(&state.store),
        Arc::clone(&state.next_packet_id),
    ));
    let join = pipeline
        .spawn(
            state.capture.frames(),
            interfaces.clone(),
            dropped,
            Arc::clone(&stop),
        )
        .map_err(|error| {
            let _ = state.capture.stop();
            format!("could not start the decode pipeline: {error}")
        })?;

    if let Ok(mut slot) = state.pipeline.lock() {
        *slot = Some(PipelineHandle {
            stop,
            join: Some(join),
        });
    }

    tracing::info!(interfaces = interfaces.len(), "capture started");
    Ok(capture_status(&state, None))
}

#[tauri::command]
pub fn stop_capture(state: State<'_, AppState>) -> Result<CaptureStatusDto, String> {
    match state.capture.stop() {
        Ok(()) | Err(crate::capture::CaptureError::NotRunning) => {}
        Err(error) => return Err(error.to_string()),
    }
    state.shutdown_pipeline();
    Ok(capture_status(&state, None))
}

/// Full layer breakdown for one packet, served from the in-memory ring. Attribution is
/// read from the owning flow rather than the stored frame, so an owner resolved after
/// the frame was decoded still appears.
#[tauri::command]
pub fn get_packet_detail(
    state: State<'_, AppState>,
    packet_id: u64,
) -> Result<PacketDetailDto, String> {
    let packet = state
        .ring
        .get(packet_id)
        .ok_or_else(|| "this packet is no longer in the live buffer".to_owned())?;
    let attribution = packet
        .flow_id
        .and_then(|flow_id| state.tracker.attribution_of(flow_id))
        .unwrap_or_default();
    Ok(PacketDetailDto::from_packet(&packet, &attribution))
}

/// Registers a webview channel for batched packet/stat pushes.
#[tauri::command]
pub fn subscribe_packet_stream(state: State<'_, AppState>, channel: Channel<StreamMessage>) {
    state.hub.register(channel);
}

fn capture_status(state: &State<'_, AppState>, error: Option<String>) -> CaptureStatusDto {
    CaptureStatusDto {
        running: state.capture.is_running(),
        interfaces: state
            .capture
            .active_interfaces()
            .iter()
            .map(CaptureInterfaceDto::from_interface)
            .collect(),
        error,
    }
}

/// The webview is untrusted: only accept plausible Npcap device ids and cap the count
/// so a malformed payload cannot spawn unbounded threads.
fn validate_interface_ids(ids: Vec<String>) -> Result<Vec<String>, String> {
    const MAX_INTERFACES: usize = 16;
    const MAX_ID_LEN: usize = 256;

    if ids.len() > MAX_INTERFACES {
        return Err(format!(
            "at most {MAX_INTERFACES} interfaces may be selected"
        ));
    }
    for id in &ids {
        if id.is_empty() || id.len() > MAX_ID_LEN {
            return Err("interface id has an invalid length".to_owned());
        }
        if !id
            .chars()
            .all(|character| character.is_ascii_graphic() || character == ' ')
        {
            return Err("interface id contains unsupported characters".to_owned());
        }
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::validate_interface_ids;

    #[test]
    fn accepts_npcap_device_ids() {
        let ids = vec![r"\Device\NPF_{05D11418-E601-4BFB-8C74-ABC40717490A}".to_owned()];
        assert_eq!(
            validate_interface_ids(ids.clone()).expect("valid ids should pass"),
            ids
        );
    }

    #[test]
    fn rejects_empty_and_oversized_ids() {
        assert!(validate_interface_ids(vec![String::new()]).is_err());
        assert!(validate_interface_ids(vec!["x".repeat(257)]).is_err());
    }

    #[test]
    fn rejects_control_characters() {
        assert!(validate_interface_ids(vec!["\u{0}device".to_owned()]).is_err());
    }

    #[test]
    fn rejects_too_many_interfaces() {
        let ids = (0..17).map(|index| format!("device{index}")).collect();
        assert!(validate_interface_ids(ids).is_err());
    }
}
