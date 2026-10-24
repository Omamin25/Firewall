use std::{net::IpAddr, sync::Arc};

use chrono::{DateTime, Utc};

/// A single captured Ethernet frame handed from a capture thread to the decoder.
/// `data` is a copy because the pcap buffer is only valid until the next read.
#[derive(Clone, Debug)]
pub struct RawFrame {
    pub interface: Arc<str>,
    pub timestamp: DateTime<Utc>,
    pub data: Vec<u8>,
    /// Wire length, which can exceed `data.len()` when the snaplen truncated the frame.
    pub original_len: usize,
}

/// A capturable network interface as reported by the platform backend.
#[derive(Clone, Debug)]
pub struct CaptureInterface {
    /// Platform device identifier (on Windows, `\Device\NPF_{GUID}`).
    pub id: String,
    /// Human-readable adapter description, falling back to the id.
    pub name: String,
    pub addresses: Vec<IpAddr>,
    pub is_loopback: bool,
    pub is_up: bool,
    pub is_wireless: bool,
    /// True for adapters that carry real host traffic — used for auto-selection.
    /// Excludes loopback and the address-less WAN Miniport pseudo-adapters Windows
    /// reports alongside physical NICs.
    pub is_recommended: bool,
}

impl CaptureInterface {
    /// IPv4/IPv6 addresses that identify this host, used to infer packet direction.
    pub fn routable_addresses(&self) -> Vec<IpAddr> {
        self.addresses
            .iter()
            .copied()
            .filter(|address| !address.is_unspecified())
            .collect()
    }
}
