//! Wire-value formatting shared by the packet DTOs. Names are resolved in Rust so the
//! webview never maps protocol numbers to labels itself (`architecture.md` §5 rule 6).

pub fn format_mac(mac: &[u8; 6]) -> String {
    mac.iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

pub fn ether_type_name(ether_type: u16) -> String {
    let name = match ether_type {
        0x0800 => "IPv4",
        0x0806 => "ARP",
        0x86dd => "IPv6",
        0x8100 => "VLAN",
        0x8035 => "RARP",
        0x888e => "EAPOL",
        _ => return format!("0x{ether_type:04x}"),
    };
    format!("{name} (0x{ether_type:04x})")
}

pub fn ip_protocol_name(protocol: u8) -> String {
    let name = match protocol {
        1 => "ICMP",
        2 => "IGMP",
        6 => "TCP",
        17 => "UDP",
        41 => "IPv6",
        58 => "ICMPv6",
        _ => return protocol.to_string(),
    };
    format!("{name} ({protocol})")
}

pub fn arp_operation_name(operation: u16) -> String {
    match operation {
        1 => "Request (1)".to_owned(),
        2 => "Reply (2)".to_owned(),
        _ => operation.to_string(),
    }
}

pub fn icmp_description(kind: u8) -> &'static str {
    match kind {
        0 => "Echo Reply",
        3 => "Destination Unreachable",
        8 => "Echo Request",
        11 => "Time Exceeded",
        128 => "Echo Request (v6)",
        129 => "Echo Reply (v6)",
        135 => "Neighbor Solicitation",
        136 => "Neighbor Advertisement",
        _ => "ICMP",
    }
}

pub fn igmp_description(kind: u8) -> &'static str {
    match kind {
        0x11 => "Membership Query",
        0x12 => "Membership Report v1",
        0x16 => "Membership Report v2",
        0x17 => "Leave Group",
        0x22 => "Membership Report v3",
        _ => "IGMP",
    }
}
