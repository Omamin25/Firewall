//! Per-layer inspector DTOs. Split from `packet.rs` so each file stays small
//! (`code_standards.md` §1); the discriminated unions mirror the TS types one-for-one.

use serde::{Deserialize, Serialize};

use super::names::{
    arp_operation_name, ether_type_name, format_mac, icmp_description, igmp_description,
    ip_protocol_name,
};
use crate::model::layers::{L2Detail, L3Detail, L4Detail, L7Detail};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkLayerDto {
    pub source_mac: String,
    pub destination_mac: String,
    pub ether_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum NetworkLayerDto {
    Ipv4 {
        source: String,
        destination: String,
        ttl: u8,
        protocol: String,
        #[serde(rename = "dontFragment")]
        dont_fragment: bool,
        #[serde(rename = "moreFragments")]
        more_fragments: bool,
        #[serde(rename = "fragmentOffset")]
        fragment_offset: u16,
    },
    Ipv6 {
        source: String,
        destination: String,
        #[serde(rename = "hopLimit")]
        hop_limit: u8,
        #[serde(rename = "nextHeader")]
        next_header: String,
    },
    Arp {
        operation: String,
        #[serde(rename = "senderIp")]
        sender_ip: String,
        #[serde(rename = "targetIp")]
        target_ip: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum TransportLayerDto {
    Tcp {
        #[serde(rename = "sourcePort")]
        source_port: u16,
        #[serde(rename = "destinationPort")]
        destination_port: u16,
        sequence: u32,
        acknowledgment: u32,
        flags: Vec<String>,
        window: u16,
    },
    Udp {
        #[serde(rename = "sourcePort")]
        source_port: u16,
        #[serde(rename = "destinationPort")]
        destination_port: u16,
        length: u16,
    },
    Icmp {
        kind: u8,
        code: u8,
        description: String,
    },
    Igmp {
        kind: u8,
        description: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ApplicationLayerDto {
    Dns {
        #[serde(rename = "isResponse")]
        is_response: bool,
        questions: Vec<String>,
        answers: Vec<String>,
    },
    Http {
        method: Option<String>,
        host: Option<String>,
        path: Option<String>,
        status: Option<u16>,
    },
    Tls {
        sni: Option<String>,
        version: Option<String>,
        handshake: String,
    },
    Quic {
        sni: Option<String>,
        #[serde(rename = "longHeader")]
        long_header: bool,
    },
    Dhcp {
        #[serde(rename = "messageType")]
        message_type: String,
    },
    Mdns {
        #[serde(rename = "isResponse")]
        is_response: bool,
        questions: Vec<String>,
    },
    Opaque {
        hint: String,
    },
}

pub(super) fn link_dto(l2: &L2Detail) -> LinkLayerDto {
    LinkLayerDto {
        source_mac: format_mac(&l2.source_mac),
        destination_mac: format_mac(&l2.dest_mac),
        ether_type: ether_type_name(l2.ether_type),
    }
}

pub(super) fn network_dto(l3: &L3Detail) -> NetworkLayerDto {
    match l3 {
        L3Detail::Ipv4 {
            source,
            dest,
            ttl,
            protocol,
            dont_fragment,
            more_fragments,
            fragment_offset,
        } => NetworkLayerDto::Ipv4 {
            source: source.to_string(),
            destination: dest.to_string(),
            ttl: *ttl,
            protocol: ip_protocol_name(*protocol),
            dont_fragment: *dont_fragment,
            more_fragments: *more_fragments,
            fragment_offset: *fragment_offset,
        },
        L3Detail::Ipv6 {
            source,
            dest,
            hop_limit,
            next_header,
        } => NetworkLayerDto::Ipv6 {
            source: source.to_string(),
            destination: dest.to_string(),
            hop_limit: *hop_limit,
            next_header: ip_protocol_name(*next_header),
        },
        L3Detail::Arp {
            operation,
            sender_ip,
            target_ip,
        } => NetworkLayerDto::Arp {
            operation: arp_operation_name(*operation),
            sender_ip: sender_ip.to_string(),
            target_ip: target_ip.to_string(),
        },
    }
}

pub(super) fn transport_dto(l4: &L4Detail) -> TransportLayerDto {
    match l4 {
        L4Detail::Tcp {
            source_port,
            dest_port,
            sequence,
            acknowledgment,
            flags,
            window,
        } => TransportLayerDto::Tcp {
            source_port: *source_port,
            destination_port: *dest_port,
            sequence: *sequence,
            acknowledgment: *acknowledgment,
            flags: flags.labels().into_iter().map(str::to_owned).collect(),
            window: *window,
        },
        L4Detail::Udp {
            source_port,
            dest_port,
            length,
        } => TransportLayerDto::Udp {
            source_port: *source_port,
            destination_port: *dest_port,
            length: *length,
        },
        L4Detail::Icmp { kind, code } => TransportLayerDto::Icmp {
            kind: *kind,
            code: *code,
            description: icmp_description(*kind).to_owned(),
        },
        L4Detail::Igmp { kind } => TransportLayerDto::Igmp {
            kind: *kind,
            description: igmp_description(*kind).to_owned(),
        },
    }
}

pub(super) fn application_dto(l7: &L7Detail) -> ApplicationLayerDto {
    match l7 {
        L7Detail::Dns {
            is_response,
            questions,
            answers,
        } => ApplicationLayerDto::Dns {
            is_response: *is_response,
            questions: questions.clone(),
            answers: answers.clone(),
        },
        L7Detail::Http {
            method,
            host,
            path,
            status,
        } => ApplicationLayerDto::Http {
            method: method.clone(),
            host: host.clone(),
            path: path.clone(),
            status: *status,
        },
        L7Detail::Tls {
            sni,
            version,
            handshake,
        } => ApplicationLayerDto::Tls {
            sni: sni.clone(),
            version: version.clone(),
            handshake: (*handshake).to_owned(),
        },
        L7Detail::Quic { sni, long_header } => ApplicationLayerDto::Quic {
            sni: sni.clone(),
            long_header: *long_header,
        },
        L7Detail::Dhcp { message_type } => ApplicationLayerDto::Dhcp {
            message_type: (*message_type).to_owned(),
        },
        L7Detail::Mdns {
            is_response,
            questions,
        } => ApplicationLayerDto::Mdns {
            is_response: *is_response,
            questions: questions.clone(),
        },
        L7Detail::Opaque { hint } => ApplicationLayerDto::Opaque { hint: hint.clone() },
    }
}
