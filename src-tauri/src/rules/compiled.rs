//! Rule matching. [`CompiledRule`] is the pure, table-tested unit: criteria are parsed
//! and normalised once at load so the enforcement hot path only does comparisons.

use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

use super::cidr::{IpMatcher, IpMatcherError};
use crate::model::{
    Direction, MatchedRule, Rule, RuleAction, RuleDirection, RuleProtocol, TransportProtocol,
};

#[derive(Debug, Error)]
#[error("rule {id} ('{name}') was skipped: {cause}")]
pub struct RuleCompileError {
    pub id: i64,
    pub name: String,
    #[source]
    pub cause: IpMatcherError,
}

/// The packet facts a rule is matched against. Borrows the attribution strings so the
/// enforcement thread does not clone them per packet.
#[derive(Clone, Copy, Debug)]
pub struct Candidate<'a> {
    pub direction: Direction,
    pub protocol: TransportProtocol,
    /// Zero for the port-less protocols and for non-initial IP fragments.
    pub local_port: u16,
    pub remote_ip: IpAddr,
    pub remote_port: u16,
    /// `None` when the owner could not be resolved. A rule that names a process or user
    /// then cannot match — an unknown owner must never be blocked by guesswork.
    pub process_path: Option<&'a str>,
    pub user_sid: Option<&'a str>,
}

/// How a rule names a process: either the full image path or the executable alone, so
/// "block telemetry.exe" works without the user hunting down its install directory.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ProcessMatcher {
    /// Lower-cased full path.
    Path(String),
    /// Lower-cased file name.
    Name(String),
}

impl ProcessMatcher {
    fn parse(value: &str) -> Self {
        let normalised = value.trim().to_lowercase();
        if normalised.contains('\\') || normalised.contains('/') {
            ProcessMatcher::Path(normalised)
        } else {
            ProcessMatcher::Name(normalised)
        }
    }

    /// Windows paths are case-insensitive, so both sides are compared lower-cased.
    fn matches(&self, path: &str) -> bool {
        let path = path.to_lowercase();
        match self {
            ProcessMatcher::Path(expected) => &path == expected,
            ProcessMatcher::Name(expected) => path
                .rsplit(['\\', '/'])
                .next()
                .is_some_and(|name| name == expected),
        }
    }
}

/// One enabled rule, ready to evaluate. Only enabled rules are compiled: a disabled rule
/// must cost the hot path nothing.
#[derive(Debug)]
pub struct CompiledRule {
    pub id: i64,
    pub name: String,
    pub action: RuleAction,
    pub priority: i64,
    direction: RuleDirection,
    protocol: RuleProtocol,
    remote_ip: Option<IpMatcher>,
    remote_port: Option<u16>,
    local_port: Option<u16>,
    process: Option<ProcessMatcher>,
    /// SIDs are case-insensitive ASCII; stored upper-cased.
    user_sid: Option<String>,
    /// Hits already written to the database.
    stored_hits: AtomicU64,
    /// Hits not yet flushed. Bumping this is the only per-packet write.
    pending_hits: AtomicU64,
}

impl CompiledRule {
    pub fn compile(rule: &Rule) -> Result<Self, RuleCompileError> {
        let remote_ip = match rule.draft.remote_ip.as_deref() {
            Some(value) => Some(IpMatcher::parse(value).map_err(|cause| RuleCompileError {
                id: rule.id,
                name: rule.draft.name.clone(),
                cause,
            })?),
            None => None,
        };
        Ok(Self {
            id: rule.id,
            name: rule.draft.name.clone(),
            action: rule.draft.action,
            priority: rule.draft.priority,
            direction: rule.draft.direction,
            protocol: rule.draft.protocol,
            remote_ip,
            remote_port: rule.draft.remote_port,
            local_port: rule.draft.local_port,
            process: rule
                .draft
                .process_path
                .as_deref()
                .map(ProcessMatcher::parse),
            user_sid: rule
                .draft
                .user_sid
                .as_deref()
                .map(|sid| sid.trim().to_uppercase()),
            stored_hits: AtomicU64::new(rule.hit_count),
            pending_hits: AtomicU64::new(0),
        })
    }

    /// True when every criterion the rule sets is satisfied. Unset criteria match
    /// anything; a criterion the candidate cannot answer never matches.
    pub fn matches(&self, candidate: &Candidate<'_>) -> bool {
        if !self.direction.matches(candidate.direction) {
            return false;
        }
        if !self.protocol.matches(candidate.protocol) {
            return false;
        }
        if let Some(matcher) = &self.remote_ip {
            if !matcher.contains(candidate.remote_ip) {
                return false;
            }
        }
        if let Some(port) = self.remote_port {
            if port != candidate.remote_port {
                return false;
            }
        }
        if let Some(port) = self.local_port {
            if port != candidate.local_port {
                return false;
            }
        }
        if let Some(matcher) = &self.process {
            match candidate.process_path {
                Some(path) if matcher.matches(path) => {}
                _ => return false,
            }
        }
        if let Some(expected) = &self.user_sid {
            match candidate.user_sid {
                Some(sid) if sid.trim().eq_ignore_ascii_case(expected) => {}
                _ => return false,
            }
        }
        true
    }

    /// True when this rule can only be decided with attribution, so the engine knows
    /// whether the hot path has to resolve an owner at all.
    pub fn needs_attribution(&self) -> bool {
        self.process.is_some() || self.user_sid.is_some()
    }

    pub fn record_hit(&self) {
        self.pending_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Total hits including the unflushed ones, so the UI never shows a counter that
    /// lags behind what the user just saw happen.
    pub fn hit_count(&self) -> u64 {
        self.stored_hits.load(Ordering::Relaxed) + self.pending_hits.load(Ordering::Relaxed)
    }

    /// Moves the unflushed hits into the stored total and hands the delta to the caller
    /// to persist. A failed write loses at most one interval of counting.
    pub fn take_hit_delta(&self) -> u64 {
        let delta = self.pending_hits.swap(0, Ordering::Relaxed);
        if delta > 0 {
            self.stored_hits.fetch_add(delta, Ordering::Relaxed);
        }
        delta
    }

    pub fn matched(&self) -> MatchedRule {
        MatchedRule {
            id: self.id,
            name: self.name.clone(),
        }
    }

    /// Evaluation order: priority first, then allow ahead of block so an allow exception
    /// at the same priority wins, then id for a stable order between identical rules.
    pub fn order_key(&self) -> (i64, u8, i64) {
        let action_rank = match self.action {
            RuleAction::Allow => 0,
            RuleAction::Block => 1,
        };
        (self.priority, action_rank, self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_matches_the_executable_in_any_directory() {
        let matcher = ProcessMatcher::parse("Telemetry.exe");
        assert!(matcher.matches(r"C:\Program Files\Vendor\telemetry.exe"));
        assert!(!matcher.matches(r"C:\Program Files\Vendor\other.exe"));
    }

    #[test]
    fn a_full_path_matches_only_that_path() {
        let matcher = ProcessMatcher::parse(r"C:\Windows\System32\svchost.exe");
        assert!(matcher.matches(r"c:\windows\system32\SVCHOST.EXE"));
        assert!(!matcher.matches(r"C:\Temp\svchost.exe"));
    }
}
