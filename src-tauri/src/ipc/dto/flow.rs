use serde::{Deserialize, Serialize};

use crate::model::Flow;

/// Row shape for the Connections table. Attribution is flattened so the UI's filter
/// predicates read the same field names on flows and packets.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDto {
    pub id: u64,
    pub first_seen: String,
    pub last_seen: String,
    pub direction: String,
    /// Most specific protocol seen on the flow (e.g. `TLS`, not `TCP`).
    pub protocol: String,
    pub protocol_token: String,
    /// Transport the flow rides on: `tcp` / `udp` / `icmp` / `igmp`.
    pub transport: String,
    pub local_address: String,
    pub local_port: Option<u16>,
    pub remote_address: String,
    pub remote_port: Option<u16>,
    pub remote_host: Option<String>,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets: u64,
    pub duration_ms: u64,
    /// False once a FIN/RST was seen; the row stays visible until it ages out.
    pub active: bool,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub process_path: Option<String>,
    pub user_account: Option<String>,
    pub user_sid: Option<String>,
    /// True once the enforcement engine dropped a packet of this conversation.
    pub blocked: bool,
    pub matched_rule_id: Option<i64>,
    /// Name of the rule that dropped it, so the row can say *why* without a second call.
    pub matched_rule_name: Option<String>,
}

impl FlowDto {
    pub fn from_flow(flow: &Flow) -> Self {
        let ported = flow.key.protocol.has_ports();
        Self {
            id: flow.id,
            first_seen: flow.first_seen.to_rfc3339(),
            last_seen: flow.last_seen.to_rfc3339(),
            direction: flow.direction.as_str().to_owned(),
            protocol: flow.protocol.as_str().to_owned(),
            protocol_token: flow.protocol.token().to_owned(),
            transport: flow.key.protocol.as_str().to_owned(),
            local_address: flow.key.local_ip.to_string(),
            local_port: ported.then_some(flow.key.local_port),
            remote_address: flow.key.remote_ip.to_string(),
            remote_port: ported.then_some(flow.key.remote_port),
            remote_host: flow.remote_host.clone(),
            bytes_sent: flow.bytes_sent,
            bytes_recv: flow.bytes_recv,
            packets: flow.packets,
            duration_ms: flow.duration_ms(),
            active: !flow.closed,
            pid: flow.attribution.pid,
            process_name: flow.attribution.process_name.clone(),
            process_path: flow.attribution.process_path.clone(),
            user_account: flow.attribution.user_account.clone(),
            user_sid: flow.attribution.user_sid.clone(),
            blocked: flow.blocked,
            matched_rule_id: flow.matched_rule.as_ref().map(|rule| rule.id),
            matched_rule_name: flow.matched_rule.as_ref().map(|rule| rule.name.clone()),
        }
    }
}

/// What the Connections view shows about the persisted `flows` table, so retention is
/// observable without shipping the whole history over IPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowHistorySummaryDto {
    pub stored_flows: u64,
    pub oldest_seen: Option<String>,
    pub newest_seen: Option<String>,
    pub retention_hours: u32,
    pub max_rows: u64,
}

/// Result of a manual history pruning operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneResultDto {
    pub removed_count: usize,
    pub summary: FlowHistorySummaryDto,
}

/// System diagnostic and resilience health status.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealthDto {
    pub is_elevated: bool,
    pub database_ready: bool,
    pub database_path: String,
    pub log_directory: String,
    pub npcap_available: bool,
    pub active_interfaces_count: usize,
    pub active_flows_count: usize,
    pub active_rules_count: usize,
    pub enforcement_status: super::rule::EnforcementStatusDto,
}
