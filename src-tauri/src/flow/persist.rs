//! Periodic history maintenance: retire stale flows, write aggregated rows, prune.
//!
//! Runs on the decode thread on a slow timer rather than a thread of its own, because
//! the row ids SQLite hands back must be written straight back onto the live flows.
//! One batched transaction every few seconds costs milliseconds, and frames keep
//! buffering in the capture channel meanwhile.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};

use super::{FlowTracker, PersistableFlow};
use crate::store::Store;

/// How often flows are retired and their counters written.
const SWEEP_INTERVAL: Duration = Duration::from_secs(5);
/// How often the retention window and row cap are enforced.
const PRUNE_INTERVAL: Duration = Duration::from_secs(60);

pub struct FlowHistoryWriter {
    store: Arc<Store>,
    tracker: Arc<FlowTracker>,
    last_sweep: Instant,
    last_prune: Instant,
}

impl FlowHistoryWriter {
    pub fn new(store: Arc<Store>, tracker: Arc<FlowTracker>) -> Self {
        Self {
            store,
            tracker,
            last_sweep: Instant::now(),
            // Prune once on the first tick so a stale database is trimmed at startup.
            last_prune: Instant::now() - PRUNE_INTERVAL,
        }
    }

    /// Runs whatever is due. Returns the ids of flows that left the live table, so the
    /// UI can drop those rows.
    pub fn tick(&mut self, now: DateTime<Utc>) -> Vec<u64> {
        if self.last_sweep.elapsed() < SWEEP_INTERVAL {
            return Vec::new();
        }
        self.last_sweep = Instant::now();

        let maintenance = self.tracker.maintain(now);
        let mut pending = maintenance.persist;
        pending.extend(self.tracker.take_unsaved());
        self.write(&pending);

        if self.last_prune.elapsed() >= PRUNE_INTERVAL {
            self.last_prune = Instant::now();
            self.prune(now);
        }
        maintenance.retired_ids
    }

    /// Final write when capture stops, so the last counters are not lost.
    pub fn flush(&mut self, now: DateTime<Utc>) {
        let maintenance = self.tracker.maintain(now);
        let mut pending = maintenance.persist;
        pending.extend(self.tracker.take_unsaved());
        self.write(&pending);
    }

    fn write(&self, pending: &[PersistableFlow]) {
        if pending.is_empty() {
            return;
        }
        match self.store.upsert_flows(pending) {
            // Ids for flows that retired in the meantime are ignored by the tracker.
            Ok(assigned) => self.tracker.assign_row_ids(&assigned),
            // History is best-effort: a failed write must never stop live capture.
            Err(error) => {
                tracing::warn!(%error, flows = pending.len(), "flow history write failed");
            }
        }
    }

    /// The policy is re-read on every prune rather than cached at construction, so a
    /// change made in Settings takes effect on the next sweep instead of only after a
    /// capture restart.
    fn prune(&self, now: DateTime<Utc>) {
        match self.store.prune_flows(self.store.retention_policy(), now) {
            Ok(0) => {}
            Ok(removed) => tracing::info!(removed, "pruned flow history"),
            Err(error) => tracing::warn!(%error, "flow history pruning failed"),
        }
    }
}
