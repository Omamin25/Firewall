use std::{
    collections::VecDeque,
    sync::{Mutex, MutexGuard},
};

use crate::model::Packet;

/// Recent decoded packets kept for the inspector. Per-packet data is memory-only and
/// never persisted (`architecture.md` §6), so this is the sole source for detail
/// lookups and the oldest entries are evicted as new frames arrive.
pub struct PacketRing {
    capacity: usize,
    packets: Mutex<VecDeque<Packet>>,
}

impl PacketRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            packets: Mutex::new(VecDeque::with_capacity(capacity.min(1024))),
        }
    }

    pub fn push(&self, packet: Packet) {
        let mut packets = self.lock();
        if packets.len() >= self.capacity {
            packets.pop_front();
        }
        packets.push_back(packet);
    }

    /// Newest-first lookup — the inspector almost always asks for a recent packet.
    pub fn get(&self, id: u64) -> Option<Packet> {
        self.lock()
            .iter()
            .rev()
            .find(|packet| packet.id == id)
            .cloned()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// A poisoned ring only means one push panicked; recovering keeps capture alive
    /// rather than taking down the engine thread.
    fn lock(&self) -> MutexGuard<'_, VecDeque<Packet>> {
        self.packets.lock().unwrap_or_else(|error| {
            tracing::warn!("packet ring lock was poisoned; recovering");
            error.into_inner()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        layers::{L2Detail, PacketLayer},
        Direction, L7Protocol,
    };

    fn packet(id: u64) -> Packet {
        Packet {
            id,
            timestamp: chrono::Utc::now(),
            interface: "test".to_owned(),
            direction: Direction::Out,
            protocol: L7Protocol::Tcp,
            remote_host: None,
            frame_len: 64,
            captured_len: 64,
            layers: PacketLayer {
                l2: L2Detail {
                    source_mac: [0; 6],
                    dest_mac: [0; 6],
                    ether_type: 0x0800,
                },
                l3: None,
                l4: None,
                l7: None,
                payload_len: 0,
            },
            flow_id: None,
        }
    }

    #[test]
    fn evicts_oldest_beyond_capacity() {
        let ring = PacketRing::new(3);
        for id in 1..=5 {
            ring.push(packet(id));
        }
        assert_eq!(ring.len(), 3);
        assert!(ring.get(1).is_none());
        assert!(ring.get(2).is_none());
        assert_eq!(ring.get(5).map(|packet| packet.id), Some(5));
    }

    #[test]
    fn newest_entries_win_lookup() {
        let ring = PacketRing::new(4);
        ring.push(packet(7));
        assert_eq!(ring.get(7).map(|packet| packet.id), Some(7));
        assert!(ring.get(8).is_none());
    }
}
