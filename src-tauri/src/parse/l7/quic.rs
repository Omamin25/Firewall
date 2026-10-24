use crate::model::L7Detail;

const HEADER_FORM_LONG: u8 = 0x80;
const FIXED_BIT: u8 = 0x40;
const LONG_PACKET_TYPE_MASK: u8 = 0x30;
const LONG_PACKET_TYPE_INITIAL: u8 = 0x00;
/// Minimum plausible size for a short-header (1-RTT) packet: flags + connection id + payload.
const MIN_SHORT_HEADER_LEN: usize = 20;
const QUIC_PORTS: [u16; 2] = [443, 80];

/// Detects QUIC and reports whether the packet uses a long header (handshake) or a
/// short header (1-RTT data).
///
/// SNI is intentionally not extracted: in QUIC the ClientHello lives inside a CRYPTO
/// frame of an Initial packet that is header-protected and AEAD-encrypted with keys
/// derived from the destination connection ID. Recovering it needs HKDF + AES-GCM,
/// which would add a crypto dependency to the capture hot path. TLS-over-TCP still
/// yields SNI directly, so the host column stays populated for most browsing.
pub fn parse(payload: &[u8], source_port: u16, dest_port: u16) -> Option<L7Detail> {
    let first = *payload.first()?;
    if first & FIXED_BIT == 0 {
        // QUIC v1 and the invariant header both require the fixed bit; without it this
        // is some other UDP protocol (or a greased/experimental version we skip).
        return None;
    }

    if first & HEADER_FORM_LONG != 0 {
        let version = u32::from_be_bytes([
            *payload.get(1)?,
            *payload.get(2)?,
            *payload.get(3)?,
            *payload.get(4)?,
        ]);
        if !is_known_version(version) {
            return None;
        }
        // Connection IDs must fit inside the datagram for this to be a real QUIC packet.
        let dcid_len = usize::from(*payload.get(5)?);
        let scid_offset = 6_usize.checked_add(dcid_len)?;
        let scid_len = usize::from(*payload.get(scid_offset)?);
        if scid_offset.checked_add(1 + scid_len)? > payload.len() {
            return None;
        }
        let _is_initial = first & LONG_PACKET_TYPE_MASK == LONG_PACKET_TYPE_INITIAL;
        return Some(L7Detail::Quic {
            sni: None,
            long_header: true,
        });
    }

    // Short headers carry no version or magic, so restrict them to the ports where
    // QUIC actually runs to avoid claiming unrelated UDP traffic.
    let on_quic_port = QUIC_PORTS.contains(&source_port) || QUIC_PORTS.contains(&dest_port);
    if on_quic_port && payload.len() >= MIN_SHORT_HEADER_LEN {
        return Some(L7Detail::Quic {
            sni: None,
            long_header: false,
        });
    }
    None
}

/// QUIC v1 (RFC 9000), v2 (RFC 9369), the draft-29 series still seen in the wild, and
/// version negotiation packets (version 0).
fn is_known_version(version: u32) -> bool {
    matches!(version, 0x0000_0000 | 0x0000_0001 | 0x6b33_43cf)
        || (0xff00_0000..=0xff00_00ff).contains(&version)
}
