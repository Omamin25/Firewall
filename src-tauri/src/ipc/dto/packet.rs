use serde::{Deserialize, Serialize};

use super::layers::{
    application_dto, link_dto, network_dto, transport_dto, ApplicationLayerDto, LinkLayerDto,
    NetworkLayerDto, TransportLayerDto,
};
use crate::model::{Attribution, Packet, PacketLayer};

/// Row shape for the Live Traffic table. Deliberately flat and small — thousands of
/// these cross the IPC boundary per second, so per-layer detail is fetched on demand
/// via `get_packet_detail`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketDto {
    pub id: u64,
    pub timestamp: String,
    pub direction: String,
    pub protocol: String,
    /// `--proto-*` token key so the UI never maps protocol names to colors itself.
    pub protocol_token: String,
    pub local_address: Option<String>,
    pub local_port: Option<u16>,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub remote_host: Option<String>,
    pub summary: String,
    pub bytes: u64,
    pub process_name: Option<String>,
    pub user_account: Option<String>,
}

/// Full layer-by-layer breakdown for the Packet Inspector.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketDetailDto {
    pub id: u64,
    pub timestamp: String,
    pub interface: String,
    pub direction: String,
    pub protocol: String,
    pub protocol_token: String,
    pub frame_len: u64,
    pub captured_len: u64,
    pub payload_len: u64,
    pub link: LinkLayerDto,
    pub network: Option<NetworkLayerDto>,
    pub transport: Option<TransportLayerDto>,
    pub application: Option<ApplicationLayerDto>,
    pub attribution: AttributionDto,
}

/// Nested attribution block for the inspector. Table rows carry the same fields flat,
/// so the UI's filter predicates work over packets and flows alike.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributionDto {
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub process_path: Option<String>,
    pub user_account: Option<String>,
    pub user_sid: Option<String>,
}

impl From<&Attribution> for AttributionDto {
    fn from(attribution: &Attribution) -> Self {
        Self {
            pid: attribution.pid,
            process_name: attribution.process_name.clone(),
            process_path: attribution.process_path.clone(),
            user_account: attribution.user_account.clone(),
            user_sid: attribution.user_sid.clone(),
        }
    }
}

impl PacketDto {
    /// `owner` comes from the packet's flow, not the frame itself — attribution is a
    /// property of the connection and can resolve after the frame was decoded.
    pub fn from_packet(packet: &Packet, owner: &Attribution) -> Self {
        Self {
            id: packet.id,
            timestamp: packet.timestamp.to_rfc3339(),
            direction: packet.direction.as_str().to_owned(),
            protocol: packet.protocol.as_str().to_owned(),
            protocol_token: packet.protocol.token().to_owned(),
            local_address: packet.local_addr().map(|address| address.to_string()),
            local_port: packet.local_port(),
            remote_address: packet.remote_addr().map(|address| address.to_string()),
            remote_port: packet.remote_port(),
            remote_host: packet.remote_host.clone(),
            summary: packet.summary(),
            bytes: packet.frame_len as u64,
            process_name: owner.process_name.clone(),
            user_account: owner.user_account.clone(),
        }
    }
}

impl PacketDetailDto {
    /// `attribution` is passed in rather than read off the packet so the inspector can
    /// show the owning flow's current owner, which is often resolved after the frame
    /// itself was decoded.
    pub fn from_packet(packet: &Packet, attribution: &Attribution) -> Self {
        let PacketLayer {
            l2,
            l3,
            l4,
            l7,
            payload_len,
        } = &packet.layers;
        Self {
            id: packet.id,
            timestamp: packet.timestamp.to_rfc3339(),
            interface: packet.interface.clone(),
            direction: packet.direction.as_str().to_owned(),
            protocol: packet.protocol.as_str().to_owned(),
            protocol_token: packet.protocol.token().to_owned(),
            frame_len: packet.frame_len as u64,
            captured_len: packet.captured_len as u64,
            payload_len: *payload_len as u64,
            link: link_dto(l2),
            network: l3.as_ref().map(network_dto),
            transport: l4.as_ref().map(transport_dto),
            application: l7.as_ref().map(application_dto),
            attribution: attribution.into(),
        }
    }
}
