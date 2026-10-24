use std::{
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use chrono::{TimeZone, Utc};
use pcap::{Capture, Device};

use super::{CaptureBackend, CaptureError, CaptureInterface, FrameSink, RawFrame};

/// Npcap-backed capture. Requires the Npcap driver (WinPcap-compatible mode) and
/// Administrator rights; both failures surface as [`CaptureError::Backend`].
pub struct NpcapBackend {
    snaplen: i32,
    read_timeout_ms: i32,
}

impl NpcapBackend {
    pub fn new(snaplen: i32, read_timeout_ms: i32) -> Self {
        Self {
            snaplen,
            read_timeout_ms,
        }
    }
}

impl CaptureBackend for NpcapBackend {
    fn list_interfaces(&self) -> Result<Vec<CaptureInterface>, CaptureError> {
        let devices = Device::list().map_err(|error| CaptureError::Backend(error.to_string()))?;
        Ok(devices.into_iter().map(describe).collect())
    }

    fn capture(
        &self,
        interface: &CaptureInterface,
        sink: FrameSink,
        stop: Arc<AtomicBool>,
    ) -> Result<(), CaptureError> {
        let mut handle = Capture::from_device(interface.id.as_str())
            .map_err(|error| CaptureError::Backend(error.to_string()))?
            .promisc(true)
            .immediate_mode(true)
            .snaplen(self.snaplen)
            .timeout(self.read_timeout_ms)
            .open()
            .map_err(|error| CaptureError::Backend(error.to_string()))?;

        let label: Arc<str> = Arc::from(interface.id.as_str());
        tracing::info!(interface = %interface.name, "capture started");

        let mut consecutive_errors = 0_u32;
        while !stop.load(Ordering::Relaxed) {
            match handle.next_packet() {
                Ok(packet) => {
                    consecutive_errors = 0;
                    sink.push(RawFrame {
                        interface: Arc::clone(&label),
                        timestamp: timestamp(packet.header.ts),
                        data: packet.data.to_vec(),
                        original_len: packet.header.len as usize,
                    });
                }
                // Expected whenever the interface is idle — loop back and re-check `stop`.
                Err(pcap::Error::TimeoutExpired) => consecutive_errors = 0,
                Err(pcap::Error::NoMorePackets) => break,
                Err(error) => {
                    consecutive_errors += 1;
                    tracing::warn!(interface = %interface.name, %error, "capture read error");
                    // A wedged or removed adapter errors continuously; give up rather
                    // than spin. Phase 6 adds re-open on interface change.
                    if consecutive_errors >= 32 {
                        return Err(CaptureError::Backend(error.to_string()));
                    }
                }
            }
        }
        Ok(())
    }
}

/// pcap reports `WAN Miniport` pseudo-adapters and the loopback shim alongside real
/// NICs. Only adapters that are up, non-loopback, and hold an assigned address carry
/// host traffic worth capturing by default.
fn describe(device: Device) -> CaptureInterface {
    let addresses: Vec<IpAddr> = device
        .addresses
        .iter()
        .map(|address| address.addr)
        .filter(|address| !address.is_unspecified())
        .collect();
    let is_loopback = device.flags.is_loopback();
    let is_up = device.flags.is_up() && device.flags.is_running();
    let has_routable_address = addresses
        .iter()
        .any(|address| !address.is_loopback() && !is_link_local(address));
    let name = device
        .desc
        .clone()
        .filter(|desc| !desc.trim().is_empty())
        .unwrap_or_else(|| device.name.clone());

    CaptureInterface {
        id: device.name,
        name,
        addresses,
        is_loopback,
        is_up,
        is_wireless: device.flags.is_wireless(),
        is_recommended: is_up && !is_loopback && has_routable_address,
    }
}

fn is_link_local(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => v4.is_link_local(),
        IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) == 0xfe80,
    }
}

/// pcap timestamps are `timeval` (seconds + microseconds) since the Unix epoch.
fn timestamp(ts: libc::timeval) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(ts.tv_sec as i64, (ts.tv_usec as u32).saturating_mul(1_000))
        .single()
        .unwrap_or_else(Utc::now)
}
