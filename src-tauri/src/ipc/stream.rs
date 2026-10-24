use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use tauri::ipc::Channel;

use super::{
    dto::{FlowDto, PacketDto, StreamMessage, StreamStatsDto},
    ring::PacketRing,
};
use crate::{
    capture::{CaptureInterface, RawFrame},
    flow::{FlowHistoryWriter, FlowTracker},
    model::{Attribution, Packet},
    parse::{self, DecodeContext},
    store::Store,
};

/// Flush cadence for batched packet pushes (~15 Hz, `architecture.md` §5 rule 2).
const FLUSH_INTERVAL: Duration = Duration::from_millis(67);
/// Upper bound on rows per batch. Excess packets are still counted in stats, folded into
/// their flows, and kept in the ring, but are not shipped to the webview — the table
/// shows a sample under extreme load rather than stalling the UI thread.
const MAX_BATCH_SIZE: usize = 256;
/// How long the decoder waits on the frame channel before checking its stop flag.
const RECV_TIMEOUT: Duration = Duration::from_millis(50);

/// Fan-out for subscribed webview channels. Channels are dropped from the list once
/// the UI side goes away (`send` fails), so a reloaded window does not leak senders.
#[derive(Default)]
pub struct StreamHub {
    channels: Mutex<Vec<Channel<StreamMessage>>>,
}

impl StreamHub {
    pub fn register(&self, channel: Channel<StreamMessage>) {
        if let Ok(mut channels) = self.channels.lock() {
            channels.push(channel);
        }
    }

    fn broadcast(&self, message: &StreamMessage) {
        let Ok(mut channels) = self.channels.lock() else {
            return;
        };
        channels.retain(|channel| match channel.send(message.clone()) {
            Ok(()) => true,
            Err(error) => {
                tracing::debug!(%error, "stream subscriber disconnected");
                false
            }
        });
    }
}

/// Rolling counters reported to the UI with each stats push.
#[derive(Default)]
struct Counters {
    packets: u64,
    bytes: u64,
    window_packets: u64,
    window_bytes: u64,
    window_started: Option<Instant>,
    last_rate: (u64, u64),
}

impl Counters {
    fn record(&mut self, bytes: usize) {
        self.packets += 1;
        self.bytes += bytes as u64;
        self.window_packets += 1;
        self.window_bytes += bytes as u64;
        self.window_started.get_or_insert_with(Instant::now);
    }

    /// Recomputes per-second rates once a full second has elapsed; between updates the
    /// previous rate is reused so the UI does not flicker between 0 and a real value.
    fn rates(&mut self) -> (u64, u64) {
        let elapsed = self
            .window_started
            .map(|start| start.elapsed())
            .unwrap_or_default();
        if elapsed >= Duration::from_secs(1) {
            let seconds = elapsed.as_secs_f64().max(f64::EPSILON);
            self.last_rate = (
                (self.window_packets as f64 / seconds) as u64,
                (self.window_bytes as f64 / seconds) as u64,
            );
            self.window_packets = 0;
            self.window_bytes = 0;
            self.window_started = Some(Instant::now());
        }
        self.last_rate
    }
}

/// Drains raw frames, decodes them, folds them into flows, and pushes batched DTOs to
/// every subscriber. Runs until `stop` is set.
pub struct PacketPipeline {
    hub: Arc<StreamHub>,
    ring: Arc<PacketRing>,
    tracker: Arc<FlowTracker>,
    store: Arc<Store>,
    next_id: Arc<AtomicU64>,
}

impl PacketPipeline {
    /// `next_packet_id` is owned by the app, not the pipeline, so a stop/start cycle
    /// cannot reissue ids that packets still sitting in the ring already use — the
    /// inspector resolves attribution through those ids.
    pub fn new(
        hub: Arc<StreamHub>,
        ring: Arc<PacketRing>,
        tracker: Arc<FlowTracker>,
        store: Arc<Store>,
        next_packet_id: Arc<AtomicU64>,
    ) -> Self {
        Self {
            hub,
            ring,
            tracker,
            store,
            next_id: next_packet_id,
        }
    }

    /// Spawns the decode+batch thread. `dropped_frames` is read from the capture engine
    /// so the UI sees capture-side loss, not just decode-side.
    pub fn spawn(
        self: Arc<Self>,
        frames: crossbeam_channel::Receiver<RawFrame>,
        interfaces: Vec<CaptureInterface>,
        dropped_frames: Arc<dyn Fn() -> u64 + Send + Sync>,
        stop: Arc<AtomicBool>,
    ) -> std::io::Result<thread::JoinHandle<()>> {
        let contexts = decode_contexts(&interfaces);
        thread::Builder::new()
            .name("cognton-decode".to_owned())
            .spawn(move || self.run(frames, contexts, dropped_frames, stop))
    }

    fn run(
        &self,
        frames: crossbeam_channel::Receiver<RawFrame>,
        contexts: HashMap<String, DecodeContext>,
        dropped_frames: Arc<dyn Fn() -> u64 + Send + Sync>,
        stop: Arc<AtomicBool>,
    ) {
        let fallback = DecodeContext::default();
        let mut batch: Vec<PacketDto> = Vec::with_capacity(MAX_BATCH_SIZE);
        let mut counters = Counters::default();
        let mut last_flush = Instant::now();
        let mut history =
            FlowHistoryWriter::new(Arc::clone(&self.store), Arc::clone(&self.tracker));

        while !stop.load(Ordering::Relaxed) {
            match frames.recv_timeout(RECV_TIMEOUT) {
                Ok(frame) => {
                    let context = contexts.get(frame.interface.as_ref()).unwrap_or(&fallback);
                    if let Some(mut packet) = self.decode(&frame, context) {
                        counters.record(packet.frame_len);
                        let shipping = batch.len() < MAX_BATCH_SIZE;
                        let owner = self.tracker.observe(&mut packet, shipping);
                        if shipping {
                            batch.push(PacketDto::from_packet(
                                &packet,
                                owner.as_ref().unwrap_or(&Attribution::EMPTY),
                            ));
                        }
                        self.ring.push(packet);
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }

            if last_flush.elapsed() >= FLUSH_INTERVAL {
                self.flush(&mut batch, &mut counters, dropped_frames.as_ref());
                last_flush = Instant::now();
                // Retirement, history writes, and pruning run on their own slower
                // timers inside `tick`; checking at flush cadence keeps the wall-clock
                // read off the per-frame path.
                let retired = history.tick(Utc::now());
                if !retired.is_empty() {
                    self.hub.broadcast(&StreamMessage::FlowsRetired(retired));
                }
            }
        }

        // Final flush so the last packets and counters before a stop still land.
        self.flush(&mut batch, &mut counters, dropped_frames.as_ref());
        history.flush(Utc::now());
        tracing::info!("decode pipeline stopped");
    }

    fn decode(&self, frame: &RawFrame, context: &DecodeContext) -> Option<Packet> {
        let decoded = parse::decode(&frame.data, context)?;
        Some(Packet {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            timestamp: frame.timestamp,
            interface: frame.interface.to_string(),
            direction: decoded.direction,
            protocol: decoded.protocol,
            remote_host: decoded.remote_host,
            frame_len: frame.original_len.max(frame.data.len()),
            captured_len: frame.data.len(),
            layers: decoded.layers,
            flow_id: None,
        })
    }

    fn flush(
        &self,
        batch: &mut Vec<PacketDto>,
        counters: &mut Counters,
        dropped_frames: &(dyn Fn() -> u64 + Send + Sync),
    ) {
        if !batch.is_empty() {
            self.hub
                .broadcast(&StreamMessage::Packets(std::mem::take(batch)));
            batch.reserve(MAX_BATCH_SIZE);
        }
        // Only flows whose counters or owner changed are sent; the table is patched.
        let changed = self.tracker.drain_dirty();
        if !changed.is_empty() {
            self.hub.broadcast(&StreamMessage::Flows(
                changed.iter().map(FlowDto::from_flow).collect(),
            ));
        }
        let (packets_per_second, bytes_per_second) = counters.rates();
        self.hub.broadcast(&StreamMessage::Stats(StreamStatsDto {
            packets_captured: counters.packets,
            bytes_captured: counters.bytes,
            packets_dropped: dropped_frames(),
            packets_per_second,
            bytes_per_second,
            active_flows: self.tracker.active_count() as u64,
        }));
    }
}

/// Per-interface direction hints, keyed by device id.
fn decode_contexts(interfaces: &[CaptureInterface]) -> HashMap<String, DecodeContext> {
    interfaces
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
        .collect()
}
