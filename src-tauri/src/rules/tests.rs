//! Table-driven rule-engine tests (`code_standards.md` §5): rule × packet → verdict.

use std::net::IpAddr;

use chrono::Utc;

use super::*;
use crate::model::{
    Direction, Rule, RuleAction, RuleDirection, RuleDraft, RuleProtocol, TransportProtocol,
};

fn ip(value: &str) -> IpAddr {
    value.parse().expect("test address should parse")
}

/// A rule with no criteria set: every field is widened, so each test narrows only what
/// it is about.
fn draft(name: &str, action: RuleAction) -> RuleDraft {
    RuleDraft {
        name: name.to_owned(),
        action,
        direction: RuleDirection::Both,
        protocol: RuleProtocol::Any,
        remote_ip: None,
        remote_port: None,
        local_port: None,
        process_path: None,
        user_sid: None,
        enabled: true,
        priority: 100,
    }
}

fn rule(id: i64, draft: RuleDraft) -> Rule {
    Rule {
        id,
        draft,
        hit_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// An outbound HTTPS packet from a known process, which each case then varies.
fn candidate<'a>(process_path: Option<&'a str>, user_sid: Option<&'a str>) -> Candidate<'a> {
    Candidate {
        direction: Direction::Out,
        protocol: TransportProtocol::Tcp,
        local_port: 51_000,
        remote_ip: ip("93.184.216.34"),
        remote_port: 443,
        process_path,
        user_sid,
    }
}

fn hot_set(rules: Vec<Rule>) -> RuleSet {
    let set = RuleSet::default();
    assert!(
        set.load(&rules).is_empty(),
        "fixtures should compile cleanly"
    );
    set
}

#[test]
fn no_rules_means_allow_with_no_match() {
    let set = RuleSet::default();
    let verdict = set.evaluate(&candidate(None, None));
    assert_eq!(verdict, Verdict::allow());
    assert!(!set.has_block_rules());
    assert!(!set.needs_attribution());
}

/// One row of the criteria table: a label, the narrowing to apply, and whether the
/// fixture packet should be blocked afterwards.
type CriteriaCase = (&'static str, Box<dyn Fn(&mut RuleDraft)>, bool);

#[test]
fn criteria_table() {
    let base = candidate(Some(r"C:\Apps\browser.exe"), Some("S-1-5-21-7-1001"));
    let cases: Vec<CriteriaCase> = vec![
        (
            "no criteria matches everything",
            Box::new(|_: &mut RuleDraft| {}),
            true,
        ),
        (
            "matching remote address",
            Box::new(|draft: &mut RuleDraft| draft.remote_ip = Some("93.184.216.34".to_owned())),
            true,
        ),
        (
            "matching remote CIDR",
            Box::new(|draft: &mut RuleDraft| draft.remote_ip = Some("93.184.0.0/16".to_owned())),
            true,
        ),
        (
            "non-matching remote CIDR",
            Box::new(|draft: &mut RuleDraft| draft.remote_ip = Some("10.0.0.0/8".to_owned())),
            false,
        ),
        (
            "matching remote port",
            Box::new(|draft: &mut RuleDraft| draft.remote_port = Some(443)),
            true,
        ),
        (
            "non-matching remote port",
            Box::new(|draft: &mut RuleDraft| draft.remote_port = Some(80)),
            false,
        ),
        (
            "matching local port",
            Box::new(|draft: &mut RuleDraft| draft.local_port = Some(51_000)),
            true,
        ),
        (
            "non-matching local port",
            Box::new(|draft: &mut RuleDraft| draft.local_port = Some(51_001)),
            false,
        ),
        (
            "matching protocol",
            Box::new(|draft: &mut RuleDraft| draft.protocol = RuleProtocol::Tcp),
            true,
        ),
        (
            "non-matching protocol",
            Box::new(|draft: &mut RuleDraft| draft.protocol = RuleProtocol::Udp),
            false,
        ),
        (
            "matching direction",
            Box::new(|draft: &mut RuleDraft| draft.direction = RuleDirection::Out),
            true,
        ),
        (
            "non-matching direction",
            Box::new(|draft: &mut RuleDraft| draft.direction = RuleDirection::In),
            false,
        ),
        (
            "process by bare name",
            Box::new(|draft: &mut RuleDraft| draft.process_path = Some("browser.exe".to_owned())),
            true,
        ),
        (
            "process by full path",
            Box::new(|draft: &mut RuleDraft| {
                draft.process_path = Some(r"c:\apps\browser.exe".to_owned())
            }),
            true,
        ),
        (
            "different process",
            Box::new(|draft: &mut RuleDraft| draft.process_path = Some("other.exe".to_owned())),
            false,
        ),
        (
            "matching user SID",
            Box::new(|draft: &mut RuleDraft| draft.user_sid = Some("s-1-5-21-7-1001".to_owned())),
            true,
        ),
        (
            "different user SID",
            Box::new(|draft: &mut RuleDraft| draft.user_sid = Some("S-1-5-18".to_owned())),
            false,
        ),
        (
            "all criteria together",
            Box::new(|draft: &mut RuleDraft| {
                draft.direction = RuleDirection::Out;
                draft.protocol = RuleProtocol::Tcp;
                draft.remote_ip = Some("93.184.216.0/24".to_owned());
                draft.remote_port = Some(443);
                draft.local_port = Some(51_000);
                draft.process_path = Some("browser.exe".to_owned());
                draft.user_sid = Some("S-1-5-21-7-1001".to_owned());
            }),
            true,
        ),
        (
            "one criterion off blocks nothing",
            Box::new(|draft: &mut RuleDraft| {
                draft.remote_port = Some(443);
                draft.process_path = Some("other.exe".to_owned());
            }),
            false,
        ),
    ];

    for (case, mutate, expect_block) in cases {
        let mut criteria = draft(case, RuleAction::Block);
        mutate(&mut criteria);
        let set = hot_set(vec![rule(1, criteria)]);
        assert_eq!(
            set.evaluate(&base).is_block(),
            expect_block,
            "case '{case}' should {}block",
            if expect_block { "" } else { "not " }
        );
    }
}

#[test]
fn a_rule_naming_a_process_cannot_match_an_unattributed_packet() {
    let mut criteria = draft("block telemetry", RuleAction::Block);
    criteria.process_path = Some("telemetry.exe".to_owned());
    let set = hot_set(vec![rule(1, criteria)]);

    assert!(set.needs_attribution());
    assert!(!set.evaluate(&candidate(None, None)).is_block());
    assert!(set
        .evaluate(&candidate(Some(r"C:\telemetry.exe"), None))
        .is_block());
}

#[test]
fn a_rule_naming_a_user_cannot_match_an_unattributed_packet() {
    let mut criteria = draft("block service account", RuleAction::Block);
    criteria.user_sid = Some("S-1-5-18".to_owned());
    let set = hot_set(vec![rule(1, criteria)]);

    assert!(!set.evaluate(&candidate(None, None)).is_block());
    assert!(set.evaluate(&candidate(None, Some("S-1-5-18"))).is_block());
}

#[test]
fn lower_priority_wins_regardless_of_action() {
    let mut allow = draft("allow one host", RuleAction::Allow);
    allow.priority = 10;
    allow.remote_ip = Some("93.184.216.34".to_owned());
    let mut block = draft("block the range", RuleAction::Block);
    block.priority = 50;
    block.remote_ip = Some("93.184.0.0/16".to_owned());

    let set = hot_set(vec![rule(1, block.clone()), rule(2, allow.clone())]);
    let verdict = set.evaluate(&candidate(None, None));
    assert_eq!(verdict.action, RuleAction::Allow);
    assert_eq!(
        verdict.rule.map(|matched| matched.name),
        Some("allow one host".to_owned())
    );

    // Flip the priorities and the block must win instead.
    let mut allow_last = allow;
    allow_last.priority = 90;
    let set = hot_set(vec![rule(1, block), rule(2, allow_last)]);
    assert!(set.evaluate(&candidate(None, None)).is_block());
}

#[test]
fn allow_beats_block_at_the_same_priority() {
    let mut allow = draft("allow", RuleAction::Allow);
    allow.remote_port = Some(443);
    let mut block = draft("block", RuleAction::Block);
    block.remote_port = Some(443);

    // Registered block-first so ordering, not insertion, decides.
    let set = hot_set(vec![rule(1, block), rule(2, allow)]);
    assert_eq!(
        set.evaluate(&candidate(None, None)).action,
        RuleAction::Allow
    );
}

#[test]
fn disabled_rules_never_reach_the_hot_set() {
    let mut criteria = draft("disabled block", RuleAction::Block);
    criteria.enabled = false;
    let set = hot_set(vec![rule(1, criteria)]);

    assert!(set.is_empty());
    assert!(!set.has_block_rules());
    assert!(!set.evaluate(&candidate(None, None)).is_block());
}

#[test]
fn a_malformed_remote_ip_is_reported_and_the_rest_still_load() {
    let mut broken = draft("broken", RuleAction::Block);
    broken.remote_ip = Some("999.1.1.1".to_owned());
    let mut good = draft("good", RuleAction::Block);
    good.remote_port = Some(443);

    let set = RuleSet::default();
    let rejected = set.load(&[rule(1, broken), rule(2, good)]);

    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].id, 1);
    assert_eq!(set.len(), 1);
    assert!(set.evaluate(&candidate(None, None)).is_block());
}

#[test]
fn hits_accumulate_in_memory_and_flush_as_deltas() {
    let mut criteria = draft("count me", RuleAction::Block);
    criteria.remote_port = Some(443);
    let mut stored = rule(7, criteria);
    stored.hit_count = 5;
    let set = hot_set(vec![stored]);

    for _ in 0..3 {
        assert!(set.evaluate(&candidate(None, None)).is_block());
    }
    // The live total includes the unflushed hits so the UI never lags reality.
    assert_eq!(set.hit_counts(), vec![(7, 8)]);
    assert_eq!(set.take_hit_deltas(), vec![(7, 3)]);
    assert!(set.take_hit_deltas().is_empty());
    assert_eq!(set.hit_counts(), vec![(7, 8)]);
}

#[test]
fn reloading_replaces_the_set_and_bumps_the_generation() {
    let set = RuleSet::default();
    let before = set.generation();
    set.load(&[rule(1, draft("block all", RuleAction::Block))]);
    assert!(set.has_block_rules());
    assert_eq!(set.block_rule_count(), 1);

    set.load(&[rule(1, draft("allow all", RuleAction::Allow))]);
    assert!(!set.has_block_rules());
    assert_eq!(set.generation(), before + 2);
}

#[test]
fn icmp_flows_are_matchable_by_any_protocol_only() {
    let icmp = Candidate {
        protocol: TransportProtocol::Icmp,
        local_port: 0,
        remote_port: 0,
        ..candidate(None, None)
    };
    let mut criteria = draft("block pings", RuleAction::Block);
    criteria.protocol = RuleProtocol::Icmp;
    assert!(hot_set(vec![rule(1, criteria)]).evaluate(&icmp).is_block());

    let mut ported = draft("block port 443", RuleAction::Block);
    ported.remote_port = Some(443);
    assert!(!hot_set(vec![rule(1, ported)]).evaluate(&icmp).is_block());
}
