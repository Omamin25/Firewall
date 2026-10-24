use crate::model::L7Detail;

const MAGIC_COOKIE: [u8; 4] = [0x63, 0x82, 0x53, 0x63];
const OP_REQUEST: u8 = 1;
const OP_REPLY: u8 = 2;
const OPTION_MESSAGE_TYPE: u8 = 53;
const OPTION_END: u8 = 255;
const OPTION_PAD: u8 = 0;
/// op(1) htype(1) hlen(1) hops(1) xid(4) secs(2) flags(2) ciaddr..giaddr(16) chaddr(16)
/// sname(64) file(128) = 236, then the 4-byte magic cookie.
const COOKIE_OFFSET: usize = 236;

/// Identifies a DHCP (BOOTP) message and its DHCP message type option.
pub fn parse(payload: &[u8]) -> Option<L7Detail> {
    let op = *payload.first()?;
    if op != OP_REQUEST && op != OP_REPLY {
        return None;
    }
    let cookie = payload.get(COOKIE_OFFSET..COOKIE_OFFSET + 4)?;
    if cookie != MAGIC_COOKIE {
        return None;
    }
    let options = payload.get(COOKIE_OFFSET + 4..).unwrap_or_default();
    let message_type = find_message_type(options).unwrap_or(match op {
        OP_REPLY => "DHCP Reply",
        _ => "DHCP Request",
    });
    Some(L7Detail::Dhcp { message_type })
}

fn find_message_type(options: &[u8]) -> Option<&'static str> {
    let mut cursor = 0_usize;
    while cursor < options.len() {
        let code = options[cursor];
        if code == OPTION_END {
            return None;
        }
        if code == OPTION_PAD {
            cursor += 1;
            continue;
        }
        let length = usize::from(*options.get(cursor + 1)?);
        let value_start = cursor + 2;
        let value = options.get(value_start..value_start.checked_add(length)?)?;
        if code == OPTION_MESSAGE_TYPE {
            return Some(message_type_name(*value.first()?));
        }
        cursor = value_start + length;
    }
    None
}

fn message_type_name(value: u8) -> &'static str {
    match value {
        1 => "DHCP Discover",
        2 => "DHCP Offer",
        3 => "DHCP Request",
        4 => "DHCP Decline",
        5 => "DHCP ACK",
        6 => "DHCP NAK",
        7 => "DHCP Release",
        8 => "DHCP Inform",
        _ => "DHCP",
    }
}
