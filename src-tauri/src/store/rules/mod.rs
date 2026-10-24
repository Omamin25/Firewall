//! Rule persistence. The database is the source of truth; `rules::RuleSet` is a compiled
//! projection of the enabled rows (`build_plan.md` Phase 4).

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Row};

use super::{Store, StoreError};
use crate::model::{Rule, RuleAction, RuleDirection, RuleDraft, RuleProtocol};

/// Every column, in the order [`row_to_rule`] reads them.
const COLUMNS: &str = "id, name, action, direction, protocol, remote_ip, remote_port, local_port, \
                       process_path, user_sid, enabled, priority, hit_count, created_at, updated_at";

impl Store {
    /// All rules, in the order the engine evaluates them: priority first, then id.
    pub fn list_rules(&self) -> Result<Vec<Rule>, StoreError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(&format!(
            "SELECT {COLUMNS} FROM rules ORDER BY priority ASC, id ASC"
        ))?;
        let mut rows = statement.query([])?;
        let mut rules = Vec::new();
        while let Some(row) = rows.next()? {
            rules.push(row_to_rule(row)?);
        }
        Ok(rules)
    }

    pub fn get_rule(&self, id: i64) -> Result<Option<Rule>, StoreError> {
        let connection = self.connection()?;
        let mut statement =
            connection.prepare(&format!("SELECT {COLUMNS} FROM rules WHERE id = ?1"))?;
        let mut rows = statement.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_rule(row)?)),
            None => Ok(None),
        }
    }

    pub fn create_rule(&self, draft: &RuleDraft, now: DateTime<Utc>) -> Result<Rule, StoreError> {
        let id = {
            let connection = self.connection()?;
            connection.execute(
                "INSERT INTO rules (
                     name, action, direction, protocol, remote_ip, remote_port, local_port,
                     process_path, user_sid, enabled, priority, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                params![
                    draft.name,
                    draft.action.as_str(),
                    draft.direction.as_str(),
                    draft.protocol.as_str(),
                    draft.remote_ip,
                    draft.remote_port,
                    draft.local_port,
                    draft.process_path,
                    draft.user_sid,
                    i64::from(draft.enabled),
                    draft.priority,
                    timestamp(now),
                ],
            )?;
            connection.last_insert_rowid()
        };
        self.require_rule(id)
    }

    /// Replaces every criterion. `hit_count` is deliberately preserved: editing a rule's
    /// scope does not erase the record of what it has already stopped.
    pub fn update_rule(
        &self,
        id: i64,
        draft: &RuleDraft,
        now: DateTime<Utc>,
    ) -> Result<Rule, StoreError> {
        let changed = {
            let connection = self.connection()?;
            connection.execute(
                "UPDATE rules SET name = ?2, action = ?3, direction = ?4, protocol = ?5,
                     remote_ip = ?6, remote_port = ?7, local_port = ?8, process_path = ?9,
                     user_sid = ?10, enabled = ?11, priority = ?12, updated_at = ?13
                 WHERE id = ?1",
                params![
                    id,
                    draft.name,
                    draft.action.as_str(),
                    draft.direction.as_str(),
                    draft.protocol.as_str(),
                    draft.remote_ip,
                    draft.remote_port,
                    draft.local_port,
                    draft.process_path,
                    draft.user_sid,
                    i64::from(draft.enabled),
                    draft.priority,
                    timestamp(now),
                ],
            )?
        };
        if changed == 0 {
            return Err(StoreError::RuleNotFound(id));
        }
        self.require_rule(id)
    }

    /// Enables or disables a rule without touching its criteria — the reversible switch
    /// `architecture.md` §5 rule 7 requires.
    pub fn set_rule_enabled(
        &self,
        id: i64,
        enabled: bool,
        now: DateTime<Utc>,
    ) -> Result<Rule, StoreError> {
        let changed = {
            let connection = self.connection()?;
            connection.execute(
                "UPDATE rules SET enabled = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, i64::from(enabled), timestamp(now)],
            )?
        };
        if changed == 0 {
            return Err(StoreError::RuleNotFound(id));
        }
        self.require_rule(id)
    }

    pub fn delete_rule(&self, id: i64) -> Result<(), StoreError> {
        let connection = self.connection()?;
        // Flow history keeps its `matched_rule` reference, so clear it first rather than
        // leaving rows pointing at a rule that no longer exists.
        connection.execute(
            "UPDATE flows SET matched_rule = NULL WHERE matched_rule = ?1",
            params![id],
        )?;
        let changed = connection.execute("DELETE FROM rules WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(StoreError::RuleNotFound(id));
        }
        Ok(())
    }

    /// Adds the hits counted in memory since the last flush. Batched in one transaction so
    /// the enforcement path never waits on a per-packet write.
    pub fn add_rule_hits(&self, deltas: &[(i64, u64)]) -> Result<(), StoreError> {
        if deltas.is_empty() {
            return Ok(());
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        {
            let mut statement =
                transaction.prepare("UPDATE rules SET hit_count = hit_count + ?2 WHERE id = ?1")?;
            for (id, delta) in deltas {
                statement.execute(params![id, *delta as i64])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    fn require_rule(&self, id: i64) -> Result<Rule, StoreError> {
        self.get_rule(id)?.ok_or(StoreError::RuleNotFound(id))
    }
}

/// Rows are parsed into the domain type, so a value that no longer satisfies the schema's
/// CHECK constraints surfaces as an error instead of a silently wrong rule.
fn row_to_rule(row: &Row<'_>) -> Result<Rule, StoreError> {
    let action: String = row.get(2)?;
    let direction: String = row.get(3)?;
    let protocol: Option<String> = row.get(4)?;
    let hit_count: i64 = row.get(12)?;
    Ok(Rule {
        id: row.get(0)?,
        draft: RuleDraft {
            name: row.get(1)?,
            action: RuleAction::parse(&action)?,
            direction: RuleDirection::parse(&direction)?,
            protocol: match protocol.as_deref() {
                // The column is nullable with a default, so treat a missing value as "any".
                None => RuleProtocol::Any,
                Some(value) => RuleProtocol::parse(value)?,
            },
            remote_ip: row.get(5)?,
            remote_port: row.get(6)?,
            local_port: row.get(7)?,
            process_path: row.get(8)?,
            user_sid: row.get(9)?,
            enabled: row.get::<_, i64>(10)? != 0,
            priority: row.get(11)?,
        },
        hit_count: hit_count.max(0) as u64,
        created_at: parse_timestamp(row.get(13)?)?,
        updated_at: parse_timestamp(row.get(14)?)?,
    })
}

/// Fixed-width UTC, matching the flow history so both tables sort as plain strings.
fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_timestamp(value: String) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StoreError::Malformed(format!("'{value}' is not an RFC-3339 timestamp")))
}

#[cfg(test)]
mod tests;
