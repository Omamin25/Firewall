//! In-memory flow aggregation. One entry per live conversation, keyed by
//! [`FlowKey`], with counters folded in on the decode thread.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::model::{Attribution, Flow, FlowKey, MatchedRule, Packet};

/// Live flows kept in memory. Past this the least-recently-active flows are retired.
pub const DEFAULT_CAPACITY: usize = 8_192;
/// A flow with no traffic for this long leaves the live table.
pub const IDLE_TIMEOUT_SECS: i64 = 90;
/// A flow closed by FIN/RST lingers only briefly, so the user still sees the teardown.
pub const CLOSED_LINGER_SECS: i64 = 10;
/// Attribution retries for a flow whose owner had not reached the OS connection table
/// yet. The first frame of a connection routinely beats it by a few milliseconds.
const MAX_ATTRIBUTION_ATTEMPTS: u8 = 6;
/// Safety valve on the retirement staging buffer, in case a caller stops draining it.
const MAX_RETIRED_BACKLOG: usize = 16_384;

/// A live flow plus the bookkeeping the table needs but the domain type should not carry.
struct TrackedFlow {
    flow: Flow,
    /// Row id in the `flows` table once written; drives insert-versus-update.
    row_id: Option<i64>,
    /// Counters changed since the last IPC push.
    dirty: bool,
    /// Counters changed since the last database write.
    unsaved: bool,
    attribution_attempts: u8,
}

impl TrackedFlow {
    fn new(flow: Flow) -> Self {
        Self {
            flow,
            row_id: None,
            dirty: true,
            unsaved: true,
            attribution_attempts: 0,
        }
    }

    /// True when the caller should try to resolve this flow's owner now. Consumes one
    /// attempt, so an unattributable flow stops costing lookups.
    fn wants_attribution(&mut self) -> bool {
        if self.flow.attribution.is_resolved()
            || self.attribution_attempts >= MAX_ATTRIBUTION_ATTEMPTS
        {
            return false;
        }
        self.attribution_attempts += 1;
        true
    }

    fn needs_write(&self) -> bool {
        self.unsaved || self.row_id.is_none()
    }

    fn persistable(&self) -> PersistableFlow {
        PersistableFlow {
            row_id: self.row_id,
            flow: self.flow.clone(),
        }
    }
}

/// What [`FlowTable::record`] tells the caller about the frame it just folded in.
#[derive(Debug, Default)]
pub struct Recorded {
    pub flow_id: u64,
    /// The caller should resolve the owner and hand it back via [`FlowTable::attribute`].
    pub wants_attribution: bool,
    /// Present only when the caller asked for it — cloning it per frame is wasted work
    /// for frames that never reach the UI.
    pub owner: Option<Attribution>,
}

/// A flow snapshot ready for the `flows` history table.
#[derive(Clone, Debug)]
pub struct PersistableFlow {
    /// `None` until the flow has been inserted once.
    pub row_id: Option<i64>,
    pub flow: Flow,
}

/// Flows retired from the live table, and the writes still owed for them.
#[derive(Debug, Default)]
pub struct Maintenance {
    pub persist: Vec<PersistableFlow>,
    pub retired_ids: Vec<u64>,
}

pub struct FlowTable {
    flows: HashMap<FlowKey, TrackedFlow>,
    retired: Vec<TrackedFlow>,
    next_id: u64,
    capacity: usize,
    idle_timeout: Duration,
    closed_linger: Duration,
}

impl FlowTable {
    pub fn new(capacity: usize) -> Self {
        Self {
            flows: HashMap::new(),
            retired: Vec::new(),
            next_id: 1,
            capacity: capacity.max(1),
            idle_timeout: Duration::seconds(IDLE_TIMEOUT_SECS),
            closed_linger: Duration::seconds(CLOSED_LINGER_SECS),
        }
    }

    /// Test seam: shortens the retirement windows so expiry can be exercised without
    /// waiting a minute and a half.
    #[cfg(test)]
    fn with_windows(mut self, idle: Duration, closed: Duration) -> Self {
        self.idle_timeout = idle;
        self.closed_linger = closed;
        self
    }

    pub fn len(&self) -> usize {
        self.flows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.flows.is_empty()
    }

    /// Folds one frame into its flow, creating the flow if this is its first frame.
    pub fn record(&mut self, key: &FlowKey, packet: &Packet, want_owner: bool) -> Recorded {
        if !self.flows.contains_key(key) {
            self.trim_to_capacity();
        }
        let next_id = self.next_id;
        let mut created = false;
        let tracked = self.flows.entry(key.clone()).or_insert_with(|| {
            created = true;
            TrackedFlow::new(Flow::new(next_id, key.clone(), packet))
        });
        if created {
            // `Flow::new` already folded this frame in.
            self.next_id += 1;
        } else {
            tracked.flow.record(packet);
            tracked.dirty = true;
            tracked.unsaved = true;
        }
        Recorded {
            flow_id: tracked.flow.id,
            wants_attribution: tracked.wants_attribution(),
            owner: want_owner.then(|| tracked.flow.attribution.clone()),
        }
    }

    /// Stores a resolved owner. A flow is only marked changed when the owner actually
    /// differs, so a stream of empty resolutions does not spam the UI with diffs.
    pub fn attribute(
        &mut self,
        key: &FlowKey,
        attribution: Attribution,
        want_owner: bool,
    ) -> Option<Attribution> {
        let tracked = self.flows.get_mut(key)?;
        if tracked.flow.attribution != attribution {
            tracked.flow.attribution = attribution;
            tracked.dirty = true;
            tracked.unsaved = true;
        }
        want_owner.then(|| tracked.flow.attribution.clone())
    }

    /// Records that the enforcement engine dropped a packet of this flow. Returns true
    /// only the first time, so the caller can log once per flow instead of once per
    /// dropped packet.
    ///
    /// A flow the capture path has never seen is not created here: enforcement and capture
    /// are independent, and inventing a row from an intercepted packet would show
    /// connections the visibility engine cannot describe.
    pub fn mark_blocked(&mut self, key: &FlowKey, rule: MatchedRule) -> bool {
        let Some(tracked) = self.flows.get_mut(key) else {
            return false;
        };
        let first = !tracked.flow.blocked;
        let changed = first || tracked.flow.matched_rule.as_ref() != Some(&rule);
        if changed {
            tracked.flow.blocked = true;
            tracked.flow.matched_rule = Some(rule);
            tracked.dirty = true;
            tracked.unsaved = true;
        }
        first
    }

    /// Snapshots of the flows that changed since the last call, for the batched push.
    pub fn drain_dirty(&mut self) -> Vec<Flow> {
        let mut changed = Vec::new();
        for tracked in self.flows.values_mut() {
            if tracked.dirty {
                tracked.dirty = false;
                changed.push(tracked.flow.clone());
            }
        }
        changed
    }

    /// Every live flow, newest activity first — used to paint the table on first load.
    pub fn snapshot(&self) -> Vec<Flow> {
        let mut flows: Vec<Flow> = self
            .flows
            .values()
            .map(|tracked| tracked.flow.clone())
            .collect();
        flows.sort_unstable_by_key(|flow| std::cmp::Reverse(flow.last_seen));
        flows
    }

    /// Current owner of a flow. Linear because the inspector asks rarely and a second
    /// index would have to be kept in step with every eviction.
    pub fn attribution_of(&self, flow_id: u64) -> Option<Attribution> {
        self.flows
            .values()
            .find(|tracked| tracked.flow.id == flow_id)
            .map(|tracked| tracked.flow.attribution.clone())
    }

    /// Flows whose counters have not reached the history table yet.
    pub fn take_unsaved(&mut self) -> Vec<PersistableFlow> {
        let mut pending = Vec::new();
        for tracked in self.flows.values_mut() {
            if tracked.needs_write() {
                tracked.unsaved = false;
                pending.push(tracked.persistable());
            }
        }
        pending
    }

    /// Records the row ids assigned by a first insert. Ids for flows that retired in the
    /// meantime are ignored.
    pub fn assign_row_ids(&mut self, assigned: &[(u64, i64)]) {
        if assigned.is_empty() {
            return;
        }
        for tracked in self.flows.values_mut() {
            if tracked.row_id.is_some() {
                continue;
            }
            if let Some((_, row_id)) = assigned.iter().find(|(id, _)| *id == tracked.flow.id) {
                tracked.row_id = Some(*row_id);
            }
        }
    }

    /// Retires idle and closed flows, enforces the capacity cap, and hands back the
    /// writes owed for everything that left the table.
    pub fn maintain(&mut self, now: DateTime<Utc>) -> Maintenance {
        let idle_timeout = self.idle_timeout;
        let closed_linger = self.closed_linger;
        let expired: Vec<FlowKey> = self
            .flows
            .iter()
            .filter(|(_, tracked)| {
                let idle = tracked.flow.idle_for(now);
                idle > idle_timeout || (tracked.flow.closed && idle > closed_linger)
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            if let Some(tracked) = self.flows.remove(&key) {
                self.stage_retired(tracked);
            }
        }
        self.trim_to_capacity();

        let mut maintenance = Maintenance::default();
        for tracked in self.retired.drain(..) {
            maintenance.retired_ids.push(tracked.flow.id);
            if tracked.needs_write() {
                maintenance.persist.push(tracked.persistable());
            }
        }
        maintenance
    }

    /// Drops the least-recently-active flows until there is room for a new one. Runs in
    /// one pass so a burst of new flows does not turn into repeated O(n) scans.
    fn trim_to_capacity(&mut self) {
        if self.flows.len() < self.capacity {
            return;
        }
        // Free a slice of the table rather than a single slot, amortising the sort.
        let target = self.capacity - (self.capacity / 16).max(1);
        let mut by_age: Vec<(FlowKey, DateTime<Utc>)> = self
            .flows
            .iter()
            .map(|(key, tracked)| (key.clone(), tracked.flow.last_seen))
            .collect();
        by_age.sort_unstable_by_key(|(_, last_seen)| *last_seen);
        let excess = self.flows.len().saturating_sub(target);
        for (key, _) in by_age.into_iter().take(excess) {
            if let Some(tracked) = self.flows.remove(&key) {
                self.stage_retired(tracked);
            }
        }
    }

    fn stage_retired(&mut self, tracked: TrackedFlow) {
        if self.retired.len() >= MAX_RETIRED_BACKLOG {
            tracing::warn!("retired flow backlog is full; dropping the oldest entry unsaved");
            self.retired.remove(0);
        }
        self.retired.push(tracked);
    }
}

impl Default for FlowTable {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests;
