//! Rolling flow history. Aggregated flows only — per-packet data is never persisted
//! (`architecture.md` §6).

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{Store, StoreError};
use crate::{flow::PersistableFlow, model::Flow};

/// Default rolling window. Overridable through the `flow_retention_hours` setting.
pub const DEFAULT_RETENTION_HOURS: u32 = 24;
/// Default row cap, so a busy host cannot grow the database without bound.
pub const DEFAULT_MAX_ROWS: u64 = 100_000;

const SETTING_RETENTION_HOURS: &str = "flow_retention_hours";
const SETTING_MAX_ROWS: &str = "flow_max_rows";

/// Effective retention policy, as reported to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetentionPolicy {
    pub hours: u32,
    pub max_rows: u64,
}

/// What the Connections view shows about persisted history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlowHistorySummary {
    pub stored_flows: u64,
    pub oldest_seen: Option<String>,
    pub newest_seen: Option<String>,
}

impl Store {
    /// Writes the pending flow snapshots in one transaction: a first write inserts, later
    /// writes update the same row so a long-lived connection keeps one history entry.
    /// Returns the row ids assigned to newly inserted flows.
    pub fn upsert_flows(&self, pending: &[PersistableFlow]) -> Result<Vec<(u64, i64)>, StoreError> {
        if pending.is_empty() {
            return Ok(Vec::new());
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut assigned = Vec::new();
        for entry in pending {
            match entry.row_id {
                Some(row_id) => update_flow(&transaction, row_id, &entry.flow)?,
                None => assigned.push((entry.flow.id, insert_flow(&transaction, &entry.flow)?)),
            }
        }
        transaction.commit()?;
        Ok(assigned)
    }

    /// Drops history outside the retention window and above the row cap. Returns the
    /// number of rows removed.
    pub fn prune_flows(
        &self,
        policy: RetentionPolicy,
        now: DateTime<Utc>,
    ) -> Result<usize, StoreError> {
        let cutoff = now - chrono::Duration::hours(i64::from(policy.hours));
        let connection = self.connection()?;
        let mut removed = connection.execute(
            "DELETE FROM flows WHERE last_seen < ?1",
            params![timestamp(cutoff)],
        )?;
        // Keep the newest `max_rows`; `LIMIT -1 OFFSET n` skips them and selects the rest.
        removed += connection.execute(
            "DELETE FROM flows WHERE id IN (SELECT id FROM flows ORDER BY last_seen DESC LIMIT -1 OFFSET ?1)",
            params![policy.max_rows as i64],
        )?;
        Ok(removed)
    }

    pub fn flow_history_summary(&self) -> Result<FlowHistorySummary, StoreError> {
        let connection = self.connection()?;
        let stored_flows: i64 =
            connection.query_row("SELECT COUNT(*) FROM flows", [], |row| row.get(0))?;
        let bounds: Option<(Option<String>, Option<String>)> = connection
            .query_row(
                "SELECT MIN(last_seen), MAX(last_seen) FROM flows",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (oldest_seen, newest_seen) = bounds.unwrap_or((None, None));
        Ok(FlowHistorySummary {
            stored_flows: stored_flows.max(0) as u64,
            oldest_seen,
            newest_seen,
        })
    }

    /// Retention policy from `settings`, falling back to the defaults. A malformed value
    /// is ignored rather than failing startup.
    pub fn retention_policy(&self) -> RetentionPolicy {
        RetentionPolicy {
            hours: self
                .get_setting(SETTING_RETENTION_HOURS)
                .ok()
                .flatten()
                .and_then(|value| value.parse().ok())
                .filter(|hours| *hours > 0)
                .unwrap_or(DEFAULT_RETENTION_HOURS),
            max_rows: self
                .get_setting(SETTING_MAX_ROWS)
                .ok()
                .flatten()
                .and_then(|value| value.parse().ok())
                .filter(|rows| *rows > 0)
                .unwrap_or(DEFAULT_MAX_ROWS),
        }
    }
}

fn insert_flow(transaction: &Transaction<'_>, flow: &Flow) -> Result<i64, StoreError> {
    transaction.execute(
        "INSERT INTO flows (
             first_seen, last_seen, direction, l7_protocol, local_ip, local_port,
             remote_ip, remote_port, remote_host, protocol, pid, process_path,
             user_account, user_sid, bytes_sent, bytes_recv, packets, blocked, matched_rule
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        params![
            timestamp(flow.first_seen),
            timestamp(flow.last_seen),
            flow.direction.as_str(),
            application_protocol(flow),
            flow.key.local_ip.to_string(),
            port(flow, flow.key.local_port),
            flow.key.remote_ip.to_string(),
            port(flow, flow.key.remote_port),
            flow.remote_host,
            flow.key.protocol.as_str(),
            flow.attribution.pid,
            flow.attribution.process_path,
            flow.attribution.user_account,
            flow.attribution.user_sid,
            flow.bytes_sent as i64,
            flow.bytes_recv as i64,
            flow.packets as i64,
            i64::from(flow.blocked),
            matched_rule(flow),
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

/// Only the fields that change over a flow's life are rewritten; the key and its first
/// sighting are fixed at insert time.
fn update_flow(transaction: &Transaction<'_>, row_id: i64, flow: &Flow) -> Result<(), StoreError> {
    transaction.execute(
        "UPDATE flows SET last_seen = ?2, l7_protocol = ?3, remote_host = ?4, pid = ?5,
             process_path = ?6, user_account = ?7, user_sid = ?8, bytes_sent = ?9,
             bytes_recv = ?10, packets = ?11, blocked = ?12, matched_rule = ?13
         WHERE id = ?1",
        params![
            row_id,
            timestamp(flow.last_seen),
            application_protocol(flow),
            flow.remote_host,
            flow.attribution.pid,
            flow.attribution.process_path,
            flow.attribution.user_account,
            flow.attribution.user_sid,
            flow.bytes_sent as i64,
            flow.bytes_recv as i64,
            flow.packets as i64,
            i64::from(flow.blocked),
            matched_rule(flow),
        ],
    )?;
    Ok(())
}

/// The rule that dropped this flow's traffic. A deleted rule clears the reference (see
/// `store/rules`), so history never points at a row that is gone.
fn matched_rule(flow: &Flow) -> Option<i64> {
    flow.matched_rule.as_ref().map(|rule| rule.id)
}

/// `l7_protocol` is the application label only; the transport already lives in
/// `protocol`, so an unclassified flow stores NULL rather than "TCP" twice.
fn application_protocol(flow: &Flow) -> Option<String> {
    flow.protocol
        .is_application()
        .then(|| flow.protocol.as_str().to_owned())
}

fn port(flow: &Flow, value: u16) -> Option<u16> {
    flow.key.protocol.has_ports().then_some(value)
}

/// Fixed-width UTC so history can be compared and pruned with plain string ordering.
fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests;
