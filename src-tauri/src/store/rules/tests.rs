use chrono::{Duration, Utc};

use super::*;
use crate::store::Store;

fn store() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().expect("test temp directory should be created");
    let store = Store::open(directory.path()).expect("store should open");
    (directory, store)
}

fn draft(name: &str) -> RuleDraft {
    RuleDraft {
        name: name.to_owned(),
        action: RuleAction::Block,
        direction: RuleDirection::Out,
        protocol: RuleProtocol::Tcp,
        remote_ip: Some("93.184.216.0/24".to_owned()),
        remote_port: Some(443),
        local_port: None,
        process_path: Some(r"C:\Apps\browser.exe".to_owned()),
        user_sid: None,
        enabled: true,
        priority: 50,
    }
}

#[test]
fn round_trips_every_criterion() {
    let (_directory, store) = store();
    let now = Utc::now();
    let created = store
        .create_rule(&draft("block example"), now)
        .expect("rule should save");

    assert_eq!(created.draft, draft("block example"));
    assert_eq!(created.hit_count, 0);

    let loaded = store
        .get_rule(created.id)
        .expect("lookup should succeed")
        .expect("rule should exist");
    assert_eq!(loaded, created);
    assert_eq!(
        store.list_rules().expect("list should succeed"),
        vec![created]
    );
}

#[test]
fn lists_rules_in_evaluation_order() {
    let (_directory, store) = store();
    let now = Utc::now();
    let mut low = draft("low priority");
    low.priority = 200;
    let mut high = draft("high priority");
    high.priority = 10;

    store.create_rule(&low, now).expect("rule should save");
    store.create_rule(&high, now).expect("rule should save");

    let names: Vec<String> = store
        .list_rules()
        .expect("list should succeed")
        .into_iter()
        .map(|rule| rule.draft.name)
        .collect();
    assert_eq!(names, vec!["high priority", "low priority"]);
}

#[test]
fn update_replaces_criteria_but_keeps_hits() {
    let (_directory, store) = store();
    let now = Utc::now();
    let created = store
        .create_rule(&draft("original"), now)
        .expect("rule should save");
    store
        .add_rule_hits(&[(created.id, 4)])
        .expect("hits should save");

    let mut edited = draft("renamed");
    edited.action = RuleAction::Allow;
    edited.remote_ip = None;
    edited.process_path = None;
    let later = now + Duration::seconds(5);
    let updated = store
        .update_rule(created.id, &edited, later)
        .expect("rule should update");

    assert_eq!(updated.draft, edited);
    assert_eq!(updated.hit_count, 4);
    assert_eq!(updated.created_at, created.created_at);
    assert!(updated.updated_at > created.updated_at);
}

#[test]
fn toggling_leaves_criteria_untouched() {
    let (_directory, store) = store();
    let now = Utc::now();
    let created = store
        .create_rule(&draft("toggle me"), now)
        .expect("rule should save");

    let disabled = store
        .set_rule_enabled(created.id, false, now)
        .expect("rule should toggle");
    assert!(!disabled.draft.enabled);
    assert_eq!(disabled.draft.remote_ip, created.draft.remote_ip);

    let enabled = store
        .set_rule_enabled(created.id, true, now)
        .expect("rule should toggle back");
    assert!(enabled.draft.enabled);
}

#[test]
fn hit_counts_accumulate_across_flushes() {
    let (_directory, store) = store();
    let created = store
        .create_rule(&draft("counted"), Utc::now())
        .expect("rule should save");

    store
        .add_rule_hits(&[(created.id, 3)])
        .expect("hits should save");
    store
        .add_rule_hits(&[(created.id, 7)])
        .expect("hits should save");
    // An id that no longer exists must not fail the batch for the others.
    store
        .add_rule_hits(&[(created.id, 1), (9_999, 5)])
        .expect("unknown ids should be ignored");

    let loaded = store
        .get_rule(created.id)
        .expect("lookup should succeed")
        .expect("rule should exist");
    assert_eq!(loaded.hit_count, 11);
}

#[test]
fn deleting_a_rule_reports_missing_ids() {
    let (_directory, store) = store();
    let created = store
        .create_rule(&draft("temporary"), Utc::now())
        .expect("rule should save");

    store.delete_rule(created.id).expect("rule should delete");
    assert!(store
        .get_rule(created.id)
        .expect("lookup should succeed")
        .is_none());
    assert!(matches!(
        store.delete_rule(created.id),
        Err(StoreError::RuleNotFound(_))
    ));
    assert!(matches!(
        store.set_rule_enabled(created.id, false, Utc::now()),
        Err(StoreError::RuleNotFound(_))
    ));
    assert!(matches!(
        store.update_rule(created.id, &draft("gone"), Utc::now()),
        Err(StoreError::RuleNotFound(_))
    ));
}

#[test]
fn a_stored_value_outside_the_schema_surfaces_as_an_error() {
    let (_directory, store) = store();
    let created = store
        .create_rule(&draft("tampered"), Utc::now())
        .expect("rule should save");
    // The CHECK constraints stop this through normal writes; PRAGMA lets the test prove
    // that a corrupted row is reported rather than silently mis-parsed.
    {
        let connection = store.connection().expect("connection should lock");
        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON")
            .expect("pragma should apply");
        connection
            .execute(
                "UPDATE rules SET action = 'quarantine' WHERE id = ?1",
                params![created.id],
            )
            .expect("tampering should apply");
    }
    assert!(matches!(store.list_rules(), Err(StoreError::Rule(_))));
}
