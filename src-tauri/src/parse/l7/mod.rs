mod dhcp;
mod dns;
mod http;
mod quic;
mod tls;

use crate::model::{
    layers::{L3Detail, L4Detail},
    L7Detail, L7Protocol,
};

const PORT_DNS: u16 = 53;
const PORT_HTTP: u16 = 80;
const PORT_HTTP_ALT: u16 = 8080;
const PORT_HTTPS: u16 = 443;
const PORT_DHCP_SERVER: u16 = 67;
const PORT_DHCP_CLIENT: u16 = 68;
const PORT_MDNS: u16 = 5353;

/// Runs the L7 classifiers in port-hint order, falling back to content sniffing and
/// finally to `Opaque`. Every classifier is pure and returns `None` when the payload
/// does not match, so a malformed packet can never panic the capture pipeline.
pub fn classify(l4: &L4Detail, payload: &[u8], l3: Option<&L3Detail>) -> Option<L7Detail> {
    let _ = l3;
    let (source_port, dest_port) = match l4.ports() {
        Some(ports) => ports,
        None => {
            let hint = match l4 {
                L4Detail::Igmp { .. } => "IGMP".to_owned(),
                _ => "ICMP".to_owned(),
            };
            return Some(L7Detail::Opaque { hint });
        }
    };
    let is_tcp = matches!(l4, L4Detail::Tcp { .. });

    if payload.is_empty() {
        return Some(L7Detail::Opaque {
            hint: control_hint(l4, dest_port),
        });
    }

    let matches_port = |port: u16| source_port == port || dest_port == port;

    if matches_port(PORT_MDNS) {
        if let Some(detail) = dns::parse_mdns(payload) {
            return Some(detail);
        }
    }
    if matches_port(PORT_DNS) {
        // TCP DNS prefixes the message with a 2-byte length.
        let body = if is_tcp { payload.get(2..)? } else { payload };
        if let Some(detail) = dns::parse(body) {
            return Some(detail);
        }
    }
    if matches_port(PORT_DHCP_SERVER) || matches_port(PORT_DHCP_CLIENT) {
        if let Some(detail) = dhcp::parse(payload) {
            return Some(detail);
        }
    }
    if is_tcp {
        if let Some(detail) = tls::parse(payload) {
            return Some(detail);
        }
        if let Some(detail) = http::parse(payload) {
            return Some(detail);
        }
    } else if let Some(detail) = quic::parse(payload, source_port, dest_port) {
        return Some(detail);
    }

    Some(L7Detail::Opaque {
        hint: port_hint(is_tcp, source_port, dest_port),
    })
}

/// Label shown in the protocol column when a classifier recognised the payload.
pub fn protocol_label(detail: &L7Detail) -> Option<L7Protocol> {
    Some(match detail {
        L7Detail::Dns { .. } => L7Protocol::Dns,
        L7Detail::Http { .. } => L7Protocol::Http,
        L7Detail::Tls { .. } => L7Protocol::Tls,
        L7Detail::Quic { .. } => L7Protocol::Quic,
        L7Detail::Dhcp { .. } => L7Protocol::Dhcp,
        L7Detail::Mdns { .. } => L7Protocol::Mdns,
        L7Detail::Opaque { .. } => return None,
    })
}

/// The best remote name the classifier could extract, used for the table's host column.
pub fn remote_host(detail: &L7Detail) -> Option<String> {
    match detail {
        L7Detail::Tls { sni, .. } | L7Detail::Quic { sni, .. } => sni.clone(),
        L7Detail::Http { host, .. } => host.clone(),
        L7Detail::Dns { questions, .. } | L7Detail::Mdns { questions, .. } => {
            questions.first().cloned()
        }
        L7Detail::Dhcp { .. } | L7Detail::Opaque { .. } => None,
    }
}

/// Payload-free frames are usually TCP control (SYN/ACK/FIN); name them by well-known port.
fn control_hint(l4: &L4Detail, dest_port: u16) -> String {
    match l4 {
        L4Detail::Tcp { flags, .. } => {
            let labels = flags.labels();
            if labels.is_empty() {
                format!("TCP port {dest_port}")
            } else {
                format!("TCP [{}]", labels.join(", "))
            }
        }
        L4Detail::Udp { .. } => format!("UDP port {dest_port}"),
        L4Detail::Icmp { .. } => "ICMP".to_owned(),
        L4Detail::Igmp { .. } => "IGMP".to_owned(),
    }
}

fn port_hint(is_tcp: bool, source_port: u16, dest_port: u16) -> String {
    let transport = if is_tcp { "TCP" } else { "UDP" };
    let named = [
        (PORT_HTTPS, "HTTPS"),
        (PORT_HTTP, "HTTP"),
        (PORT_HTTP_ALT, "HTTP-alt"),
    ]
    .into_iter()
    .find_map(|(port, name)| (source_port == port || dest_port == port).then_some(name));
    match named {
        Some(name) => format!("{transport} {name} (opaque)"),
        None => format!("{transport} port {dest_port}"),
    }
}
