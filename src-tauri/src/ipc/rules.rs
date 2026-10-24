//! Rule-facing commands. Every mutation follows the same three steps — write the database,
//! recompile the hot set, reconcile the enforcement handle — so a rule takes effect (or
//! stops taking effect) the moment the command returns.

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{
    dto::{EnforcementStatusDto, RuleDraftDto, RuleDto},
    ring::PacketRing,
    AppState,
};
use crate::model::{
    Flow, Rule, RuleAction, RuleDirection, RuleDraft, RuleProtocol, TransportProtocol,
};

/// How a rule is derived from a flow the user picked. Criteria the scope does not name are
/// left wide, so the rule keeps working as the connection changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleScope {
    /// This exact conversation: remote address, remote port, protocol and direction.
    Endpoint,
    /// Everything to or from that remote address.
    Host,
    /// Everything on that remote port.
    Port,
    /// Everything the owning process does.
    Process,
}

#[tauri::command]
pub fn list_rules(state: State<'_, AppState>) -> Result<Vec<RuleDto>, String> {
    let stored = state
        .store
        .list_rules()
        .map_err(|error| error.to_string())?;
    let live: HashMap<i64, u64> = state.rules.hit_counts().into_iter().collect();
    Ok(stored
        .iter()
        .map(|rule| RuleDto::from_rule(rule, live.get(&rule.id).copied()))
        .collect())
}

#[tauri::command]
pub fn create_rule(state: State<'_, AppState>, draft: RuleDraftDto) -> Result<RuleDto, String> {
    save_new_rule(&state, draft)
}

#[tauri::command]
pub fn update_rule(
    state: State<'_, AppState>,
    rule_id: i64,
    draft: RuleDraftDto,
) -> Result<RuleDto, String> {
    let draft = draft.validate()?;
    warn_if_unbounded(&draft);
    let rule = state
        .store
        .update_rule(rule_id, &draft, Utc::now())
        .map_err(|error| error.to_string())?;
    state.apply_rule_changes();
    tracing::info!(rule = %rule.draft.name, id = rule.id, "rule updated");
    Ok(state.decorate(&rule))
}

/// Enables or disables a rule. This is the reversible switch `architecture.md` §5 rule 7
/// requires: turning a block off restores traffic immediately.
#[tauri::command]
pub fn toggle_rule(
    state: State<'_, AppState>,
    rule_id: i64,
    enabled: bool,
) -> Result<RuleDto, String> {
    let rule = state
        .store
        .set_rule_enabled(rule_id, enabled, Utc::now())
        .map_err(|error| error.to_string())?;
    state.apply_rule_changes();
    tracing::info!(rule = %rule.draft.name, id = rule.id, enabled, "rule toggled");
    Ok(state.decorate(&rule))
}

#[tauri::command]
pub fn delete_rule(state: State<'_, AppState>, rule_id: i64) -> Result<(), String> {
    state
        .store
        .delete_rule(rule_id)
        .map_err(|error| error.to_string())?;
    state.apply_rule_changes();
    tracing::info!(id = rule_id, "rule deleted");
    Ok(())
}

/// Criteria prefilled from a live flow, for the create-rule form. No database write, so the
/// user can edit before committing.
#[tauri::command]
pub fn draft_rule_from_flow(
    state: State<'_, AppState>,
    flow_id: u64,
    scope: RuleScope,
    action: String,
) -> Result<RuleDraftDto, String> {
    build_draft(&state, flow_id, scope, &action)
}

/// Criteria prefilled from a packet in the inspector (`ui_rules.md` §7). The packet is
/// mapped back to the flow it belongs to, so this and [`draft_rule_from_flow`] cannot
/// produce different rules for the same conversation.
#[tauri::command]
pub fn draft_rule_from_packet(
    state: State<'_, AppState>,
    packet_id: u64,
    scope: RuleScope,
    action: String,
) -> Result<RuleDraftDto, String> {
    let flow_id = flow_id_of_packet(&state.ring, packet_id)?;
    build_draft(&state, flow_id, scope, &action)
}

/// Creates the rule a context menu would build, in one step. Shares the builder with
/// [`draft_rule_from_flow`] so the two paths cannot disagree.
#[tauri::command]
pub fn create_rule_from_flow(
    state: State<'_, AppState>,
    flow_id: u64,
    scope: RuleScope,
    action: String,
) -> Result<RuleDto, String> {
    let draft = build_draft(&state, flow_id, scope, &action)?;
    save_new_rule(&state, draft)
}

#[tauri::command]
pub fn get_enforcement_status(state: State<'_, AppState>) -> EnforcementStatusDto {
    state.enforcement.status().into()
}

fn save_new_rule(state: &AppState, draft: RuleDraftDto) -> Result<RuleDto, String> {
    let draft = draft.validate()?;
    warn_if_unbounded(&draft);
    let rule = state
        .store
        .create_rule(&draft, Utc::now())
        .map_err(|error| error.to_string())?;
    state.apply_rule_changes();
    tracing::info!(
        rule = %rule.draft.name,
        id = rule.id,
        action = rule.draft.action.as_str(),
        "rule created"
    );
    Ok(state.decorate(&rule))
}

fn build_draft(
    state: &AppState,
    flow_id: u64,
    scope: RuleScope,
    action: &str,
) -> Result<RuleDraftDto, String> {
    let action = RuleAction::parse(action).map_err(|error| error.to_string())?;
    let flow = state
        .tracker
        .snapshot()
        .into_iter()
        .find(|flow| flow.id == flow_id)
        .ok_or_else(|| "this connection is no longer live".to_owned())?;
    Ok(draft_from_flow(&flow, scope, action))
}

/// Packets that never joined a flow (ARP, and anything decoded after its conversation was
/// retired) carry no 5-tuple worth naming in a rule.
fn flow_id_of_packet(ring: &PacketRing, packet_id: u64) -> Result<u64, String> {
    ring.get(packet_id)
        .ok_or_else(|| "this packet is no longer in the live buffer".to_owned())?
        .flow_id
        .ok_or_else(|| {
            "this packet belongs to no connection, so there is nothing to match".to_owned()
        })
}

/// Process scope falls back to the endpoint when the flow has no resolved owner: a rule
/// naming an unknown process would never match anything.
fn draft_from_flow(flow: &Flow, scope: RuleScope, action: RuleAction) -> RuleDraftDto {
    let remote = flow
        .remote_host
        .clone()
        .unwrap_or_else(|| flow.key.remote_ip.to_string());
    let ported = flow.key.protocol.has_ports();
    let protocol = match flow.key.protocol {
        TransportProtocol::Tcp => RuleProtocol::Tcp,
        TransportProtocol::Udp => RuleProtocol::Udp,
        TransportProtocol::Icmp => RuleProtocol::Icmp,
        // IGMP has no rule vocabulary of its own (`architecture.md` §6).
        TransportProtocol::Igmp => RuleProtocol::Any,
    };
    let process = flow.attribution.process_path.clone();
    let scope = match scope {
        RuleScope::Process if process.is_none() => RuleScope::Endpoint,
        other => other,
    };
    let verb = if action == RuleAction::Block {
        "Block"
    } else {
        "Allow"
    };

    let mut draft = RuleDraftDto {
        name: String::new(),
        action: action.as_str().to_owned(),
        direction: RuleDirection::Both.as_str().to_owned(),
        protocol: RuleProtocol::Any.as_str().to_owned(),
        remote_ip: None,
        remote_port: None,
        local_port: None,
        process_path: None,
        user_sid: None,
        enabled: true,
        priority: 100,
    };

    match scope {
        RuleScope::Endpoint => {
            draft.name = format!("{verb} {remote}");
            draft.direction = flow.direction.as_str().to_owned();
            draft.protocol = protocol.as_str().to_owned();
            draft.remote_ip = Some(flow.key.remote_ip.to_string());
            draft.remote_port = ported.then_some(flow.key.remote_port);
        }
        RuleScope::Host => {
            draft.name = format!("{verb} host {remote}");
            draft.remote_ip = Some(flow.key.remote_ip.to_string());
        }
        RuleScope::Port => {
            draft.name = format!("{verb} port {}", flow.key.remote_port);
            draft.protocol = protocol.as_str().to_owned();
            draft.remote_port = ported.then_some(flow.key.remote_port);
        }
        RuleScope::Process => {
            let label = flow
                .attribution
                .process_name
                .clone()
                .or_else(|| process.clone())
                .unwrap_or_else(|| "process".to_owned());
            draft.name = format!("{verb} {label}");
            draft.process_path = process;
        }
    }
    draft
}

/// A rule with no criteria matches every packet. That is a legitimate "block everything"
/// posture, so it is allowed — but it is worth a line in the log.
fn warn_if_unbounded(draft: &RuleDraft) {
    if draft.action == RuleAction::Block && draft.enabled && draft.matches_everything() {
        tracing::warn!(rule = %draft.name, "a block rule with no criteria will stop all traffic");
    }
}

impl AppState {
    /// Recompiles the hot set from the database and brings the enforcement handle in line.
    fn apply_rule_changes(&self) {
        match self.store.list_rules() {
            Ok(stored) => {
                for rejected in self.rules.load(&stored) {
                    tracing::warn!(%rejected, "a rule could not be compiled");
                }
            }
            // The write succeeded, so the rule is persisted; only the hot set missed it.
            // A log line beats failing a command that did its job.
            Err(error) => tracing::error!(%error, "rules could not be reloaded"),
        }
        self.enforcement.reconcile();
    }

    fn decorate(&self, rule: &Rule) -> RuleDto {
        let live = self
            .rules
            .hit_counts()
            .into_iter()
            .find_map(|(id, hits)| (id == rule.id).then_some(hits));
        RuleDto::from_rule(rule, live)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use chrono::Utc;

    use super::*;
    use crate::model::{Attribution, Direction, FlowKey, L7Protocol};

    fn flow(process: Option<&str>) -> Flow {
        Flow {
            id: 1,
            key: FlowKey {
                protocol: TransportProtocol::Tcp,
                local_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
                local_port: 51_000,
                remote_ip: IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                remote_port: 443,
            },
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            direction: Direction::Out,
            protocol: L7Protocol::Tls,
            remote_host: Some("example.com".to_owned()),
            bytes_sent: 0,
            bytes_recv: 0,
            packets: 1,
            closed: false,
            attribution: Attribution {
                pid: process.map(|_| 4_242),
                process_name: process.map(|_| "browser.exe".to_owned()),
                process_path: process.map(str::to_owned),
                user_account: None,
                user_sid: None,
            },
            blocked: false,
            matched_rule: None,
        }
    }

    #[test]
    fn endpoint_scope_names_the_whole_conversation() {
        let draft = draft_from_flow(&flow(None), RuleScope::Endpoint, RuleAction::Block)
            .validate()
            .expect("draft should validate");
        assert_eq!(draft.name, "Block example.com");
        assert_eq!(draft.remote_ip.as_deref(), Some("93.184.216.34"));
        assert_eq!(draft.remote_port, Some(443));
        assert_eq!(draft.direction, RuleDirection::Out);
        assert_eq!(draft.protocol, RuleProtocol::Tcp);
    }

    #[test]
    fn host_scope_ignores_ports_and_direction() {
        let draft = draft_from_flow(&flow(None), RuleScope::Host, RuleAction::Block)
            .validate()
            .expect("draft should validate");
        assert_eq!(draft.remote_port, None);
        assert_eq!(draft.direction, RuleDirection::Both);
        assert_eq!(draft.protocol, RuleProtocol::Any);
    }

    #[test]
    fn port_scope_names_only_the_port_and_transport() {
        let draft = draft_from_flow(&flow(None), RuleScope::Port, RuleAction::Allow)
            .validate()
            .expect("draft should validate");
        assert_eq!(draft.name, "Allow port 443");
        assert_eq!(draft.remote_ip, None);
        assert_eq!(draft.remote_port, Some(443));
        assert_eq!(draft.action, RuleAction::Allow);
    }

    #[test]
    fn process_scope_uses_the_resolved_path() {
        let draft = draft_from_flow(
            &flow(Some(r"C:\Apps\browser.exe")),
            RuleScope::Process,
            RuleAction::Block,
        )
        .validate()
        .expect("draft should validate");
        assert_eq!(draft.name, "Block browser.exe");
        assert_eq!(draft.process_path.as_deref(), Some(r"C:\Apps\browser.exe"));
        assert_eq!(draft.remote_ip, None);
    }

    /// A rule naming an unresolved process could never match, so the scope degrades to the
    /// endpoint rather than producing a rule that silently does nothing.
    #[test]
    fn process_scope_falls_back_when_the_owner_is_unknown() {
        let draft = draft_from_flow(&flow(None), RuleScope::Process, RuleAction::Block)
            .validate()
            .expect("draft should validate");
        assert!(draft.process_path.is_none());
        assert_eq!(draft.remote_ip.as_deref(), Some("93.184.216.34"));
    }

    /// The inspector's "create rule from this packet" action resolves the packet back to
    /// its flow; the three failure modes each need their own message.
    #[test]
    fn packet_lookup_resolves_the_owning_flow() {
        use crate::model::{
            layers::{L2Detail, PacketLayer},
            Packet,
        };

        let packet = |id: u64, flow_id: Option<u64>| Packet {
            id,
            timestamp: Utc::now(),
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
            flow_id,
        };

        let ring = PacketRing::new(4);
        ring.push(packet(1, Some(77)));
        ring.push(packet(2, None));

        assert_eq!(flow_id_of_packet(&ring, 1), Ok(77));
        assert!(flow_id_of_packet(&ring, 2)
            .expect_err("a packet with no flow cannot build a rule")
            .contains("no connection"));
        assert!(flow_id_of_packet(&ring, 9)
            .expect_err("an evicted packet cannot build a rule")
            .contains("live buffer"));
    }
}
