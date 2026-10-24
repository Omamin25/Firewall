//! IPC wire types. Every struct here is `camelCase` on the wire and mirrored by a TS
//! type in `src/types/ipc.ts` (`code_standards.md` §4).

mod capture;
mod flow;
mod layers;
mod names;
mod packet;
mod rule;

use serde::{Deserialize, Serialize};

pub use capture::{CaptureInterfaceDto, CaptureStatusDto};
pub use flow::{FlowDto, FlowHistorySummaryDto, PruneResultDto, SystemHealthDto};
pub use layers::{ApplicationLayerDto, LinkLayerDto, NetworkLayerDto, TransportLayerDto};
pub use packet::{AttributionDto, PacketDetailDto, PacketDto};
pub use rule::{EnforcementStatusDto, RuleDraftDto, RuleDto};

/// Throughput and loss counters pushed alongside packet batches.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStatsDto {
    pub packets_captured: u64,
    pub bytes_captured: u64,
    pub packets_dropped: u64,
    pub packets_per_second: u64,
    pub bytes_per_second: u64,
    pub active_flows: u64,
}

/// Batched stream envelope. One `Channel` carries every variant; batching is the
/// architectural requirement in `architecture.md` §5 rule 2.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum StreamMessage {
    Packets(Vec<PacketDto>),
    /// Only flows that changed since the last flush — the table is patched, not resent.
    Flows(Vec<FlowDto>),
    /// Ids of flows that aged out of the in-memory table, so the UI can drop the rows.
    FlowsRetired(Vec<u64>),
    Stats(StreamStatsDto),
    CaptureError {
        message: String,
    },
}
