//! The hot rule set: the enabled rules held in memory and evaluated per packet.
//!
//! The database is the source of truth; this is a compiled projection of it, reloaded
//! whenever a rule changes (`build_plan.md` Phase 4). Reads take a shared lock and never
//! allocate, so the enforcement thread can evaluate on the packet path.

pub mod cidr;
pub mod compiled;

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    RwLock, RwLockReadGuard,
};

pub use cidr::{IpMatcher, IpMatcherError};
pub use compiled::{Candidate, CompiledRule, RuleCompileError};

use crate::model::{MatchedRule, Rule, RuleAction};

/// What the engine should do with a packet, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub action: RuleAction,
    /// `None` when no rule matched — the default posture (`architecture.md` §5 rule 7).
    pub rule: Option<MatchedRule>,
}

impl Verdict {
    /// Default posture: observe, do not interfere.
    pub fn allow() -> Self {
        Self {
            action: RuleAction::Allow,
            rule: None,
        }
    }

    pub fn is_block(&self) -> bool {
        self.action == RuleAction::Block
    }
}

#[derive(Default)]
pub struct RuleSet {
    /// Enabled rules only, pre-sorted into evaluation order.
    rules: RwLock<Vec<CompiledRule>>,
    /// Cached so the engine can decide whether to hold a WinDivert handle at all without
    /// taking the lock.
    has_block_rules: AtomicBool,
    /// Cached so the packet path can skip owner resolution when no rule needs it.
    needs_attribution: AtomicBool,
    /// Bumped on every reload; lets callers tell whether the set they read is still current.
    generation: AtomicU64,
}

impl RuleSet {
    /// Replaces the set with the enabled rules from `rules`. Rules whose criteria fail to
    /// compile are skipped and returned, so one malformed row cannot disable enforcement
    /// for the rest.
    pub fn load(&self, rules: &[Rule]) -> Vec<RuleCompileError> {
        let mut compiled = Vec::new();
        let mut rejected = Vec::new();
        for rule in rules.iter().filter(|rule| rule.draft.enabled) {
            match CompiledRule::compile(rule) {
                Ok(rule) => compiled.push(rule),
                Err(error) => rejected.push(error),
            }
        }
        compiled.sort_by_key(CompiledRule::order_key);

        self.has_block_rules.store(
            compiled.iter().any(|rule| rule.action == RuleAction::Block),
            Ordering::Relaxed,
        );
        self.needs_attribution.store(
            compiled.iter().any(CompiledRule::needs_attribution),
            Ordering::Relaxed,
        );
        *self.write() = compiled;
        self.generation.fetch_add(1, Ordering::Relaxed);
        rejected
    }

    /// First match wins. Ordering puts lower priorities first and allow ahead of block at
    /// the same priority, so an allow rule overrides a block exactly as `build_plan.md`
    /// specifies.
    pub fn evaluate(&self, candidate: &Candidate<'_>) -> Verdict {
        let rules = self.read();
        for rule in rules.iter() {
            if !rule.matches(candidate) {
                continue;
            }
            rule.record_hit();
            return Verdict {
                action: rule.action,
                rule: Some(rule.matched()),
            };
        }
        Verdict::allow()
    }

    /// True while at least one enabled rule can drop a packet. When false the engine
    /// closes its network-layer handle entirely, so the default posture costs nothing.
    pub fn has_block_rules(&self) -> bool {
        self.has_block_rules.load(Ordering::Relaxed)
    }

    /// True while at least one enabled rule names a process or user.
    pub fn needs_attribution(&self) -> bool {
        self.needs_attribution.load(Ordering::Relaxed)
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    pub fn len(&self) -> usize {
        self.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.read().is_empty()
    }

    pub fn block_rule_count(&self) -> usize {
        self.read()
            .iter()
            .filter(|rule| rule.action == RuleAction::Block)
            .count()
    }

    /// Live hit totals by rule id, for merging into the rows `list_rules` returns.
    pub fn hit_counts(&self) -> Vec<(i64, u64)> {
        self.read()
            .iter()
            .map(|rule| (rule.id, rule.hit_count()))
            .collect()
    }

    /// Hits accumulated since the last call, ready to be added to the stored counters.
    /// Counting in memory keeps the database off the packet path.
    pub fn take_hit_deltas(&self) -> Vec<(i64, u64)> {
        self.read()
            .iter()
            .filter_map(|rule| {
                let delta = rule.take_hit_delta();
                (delta > 0).then_some((rule.id, delta))
            })
            .collect()
    }

    /// A poisoned rule set only means one evaluation panicked; recovering keeps
    /// enforcement running instead of taking the engine down.
    fn read(&self) -> RwLockReadGuard<'_, Vec<CompiledRule>> {
        self.rules.read().unwrap_or_else(|error| {
            tracing::warn!("rule set lock was poisoned; recovering");
            error.into_inner()
        })
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Vec<CompiledRule>> {
        self.rules.write().unwrap_or_else(|error| {
            tracing::warn!("rule set lock was poisoned; recovering");
            error.into_inner()
        })
    }
}

#[cfg(test)]
mod tests;
