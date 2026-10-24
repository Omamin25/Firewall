//! Flow aggregation: turns the decoded packet stream into connections and asks the
//! attribution engine who owns each one.

pub mod persist;
pub mod table;

use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};

use crate::{
    attribution::AttributionResolver,
    model::{Attribution, Flow, FlowKey, MatchedRule, Packet},
};
pub use persist::FlowHistoryWriter;
pub use table::{FlowTable, Maintenance, PersistableFlow, DEFAULT_CAPACITY};

/// Owns the live flow table and the attribution resolver, and keeps the OS lookups off
/// the table lock. One instance per app, shared by the decode thread and IPC commands.
pub struct FlowTracker {
    table: Mutex<FlowTable>,
    resolver: Arc<AttributionResolver>,
}

impl FlowTracker {
    pub fn new(resolver: Arc<AttributionResolver>) -> Self {
        Self {
            table: Mutex::new(FlowTable::default()),
            resolver,
        }
    }

    /// Builds a tracker over a pre-sized table. Used by tests that need to assert against
    /// a specific capacity rather than the default.
    pub fn with_table(resolver: Arc<AttributionResolver>, table: FlowTable) -> Self {
        Self {
            table: Mutex::new(table),
            resolver,
        }
    }

    /// Folds `packet` into its flow and stamps `packet.flow_id`. Returns the flow's owner
    /// when `want_owner` is set; the pipeline only asks for frames it is shipping to the
    /// UI, since every other clone would be thrown away.
    pub fn observe(&self, packet: &mut Packet, want_owner: bool) -> Option<Attribution> {
        let key = FlowKey::from_packet(packet)?;
        let recorded = self.lock().record(&key, packet, want_owner);
        packet.flow_id = Some(recorded.flow_id);
        if !recorded.wants_attribution {
            return recorded.owner;
        }
        // Resolution can read the OS connection table, so it must not hold the table
        // lock (`code_standards.md` §2: never hold a lock across a syscall batch).
        let resolved = self.resolver.resolve(&key);
        self.lock().attribute(&key, resolved, want_owner)
    }

    /// Flows that changed since the last call, for the batched IPC push.
    pub fn drain_dirty(&self) -> Vec<Flow> {
        self.lock().drain_dirty()
    }

    pub fn snapshot(&self) -> Vec<Flow> {
        self.lock().snapshot()
    }

    pub fn active_count(&self) -> usize {
        self.lock().len()
    }

    /// Owner of the flow a packet belonged to. The inspector uses this rather than the
    /// packet's own copy so attribution resolved after decode still shows up.
    pub fn attribution_of(&self, flow_id: u64) -> Option<Attribution> {
        self.lock().attribution_of(flow_id)
    }

    /// Marks the flow an intercepted packet belonged to as blocked. Returns true the first
    /// time, so enforcement can log once per flow. Flows the capture path has not seen are
    /// ignored — the two engines stay independent (`architecture.md` §5 rule 4).
    pub fn mark_blocked(&self, key: &FlowKey, rule: MatchedRule) -> bool {
        self.lock().mark_blocked(key, rule)
    }

    /// Live flows whose counters have not reached the history table yet.
    pub fn take_unsaved(&self) -> Vec<PersistableFlow> {
        self.lock().take_unsaved()
    }

    pub fn assign_row_ids(&self, assigned: &[(u64, i64)]) {
        self.lock().assign_row_ids(assigned);
    }

    /// Retires idle, closed, and over-capacity flows.
    pub fn maintain(&self, now: DateTime<Utc>) -> Maintenance {
        self.lock().maintain(now)
    }

    /// A poisoned table only means one fold panicked; recovering keeps capture alive
    /// instead of taking down the decode thread.
    fn lock(&self) -> MutexGuard<'_, FlowTable> {
        self.table.lock().unwrap_or_else(|error| {
            tracing::warn!("flow table lock was poisoned; recovering");
            error.into_inner()
        })
    }
}
