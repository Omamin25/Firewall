use crate::model::L7Detail;

const RECORD_HANDSHAKE: u8 = 0x16;
const RECORD_APPLICATION_DATA: u8 = 0x17;
const RECORD_ALERT: u8 = 0x15;
const RECORD_CHANGE_CIPHER_SPEC: u8 = 0x14;

const HANDSHAKE_CLIENT_HELLO: u8 = 0x01;
const HANDSHAKE_SERVER_HELLO: u8 = 0x02;

const EXTENSION_SERVER_NAME: u16 = 0x0000;
const SNI_TYPE_HOST_NAME: u8 = 0x00;

/// Detects a TLS record and, for a ClientHello, extracts the SNI host name.
/// Only cleartext handshake metadata is read — payloads are never decrypted.
pub fn parse(payload: &[u8]) -> Option<L7Detail> {
    let content_type = *payload.first()?;
    let major = *payload.get(1)?;
    // All TLS/SSL versions in use start their record with major version 3.
    if major != 3 {
        return None;
    }
    let version = version_name(major, *payload.get(2)?);

    match content_type {
        RECORD_HANDSHAKE => {
            let body = payload.get(5..)?;
            match body.first().copied() {
                Some(HANDSHAKE_CLIENT_HELLO) => Some(L7Detail::Tls {
                    sni: client_hello_sni(body),
                    version,
                    handshake: "ClientHello",
                }),
                Some(HANDSHAKE_SERVER_HELLO) => Some(L7Detail::Tls {
                    sni: None,
                    version,
                    handshake: "ServerHello",
                }),
                Some(_) => Some(L7Detail::Tls {
                    sni: None,
                    version,
                    handshake: "Handshake",
                }),
                None => None,
            }
        }
        RECORD_APPLICATION_DATA => Some(L7Detail::Tls {
            sni: None,
            version,
            handshake: "Application Data",
        }),
        RECORD_ALERT => Some(L7Detail::Tls {
            sni: None,
            version,
            handshake: "Alert",
        }),
        RECORD_CHANGE_CIPHER_SPEC => Some(L7Detail::Tls {
            sni: None,
            version,
            handshake: "ChangeCipherSpec",
        }),
        _ => None,
    }
}

/// Walks a ClientHello body (starting at the handshake type byte) to the SNI extension.
fn client_hello_sni(handshake: &[u8]) -> Option<String> {
    // handshake_type(1) length(3) version(2) random(32)
    let mut cursor = 38_usize;
    let session_id_len = usize::from(*handshake.get(cursor)?);
    cursor = cursor.checked_add(1 + session_id_len)?;

    let cipher_suites_len = usize::from(be_u16(handshake, cursor)?);
    cursor = cursor.checked_add(2 + cipher_suites_len)?;

    let compression_len = usize::from(*handshake.get(cursor)?);
    cursor = cursor.checked_add(1 + compression_len)?;

    let extensions_len = usize::from(be_u16(handshake, cursor)?);
    cursor = cursor.checked_add(2)?;
    let extensions_end = cursor.checked_add(extensions_len)?.min(handshake.len());

    while cursor + 4 <= extensions_end {
        let extension_type = be_u16(handshake, cursor)?;
        let extension_len = usize::from(be_u16(handshake, cursor + 2)?);
        let body_start = cursor + 4;
        let body_end = body_start.checked_add(extension_len)?;
        if body_end > extensions_end {
            return None;
        }
        if extension_type == EXTENSION_SERVER_NAME {
            return server_name(handshake.get(body_start..body_end)?);
        }
        cursor = body_end;
    }
    None
}

/// server_name extension: list_length(2) then entries of type(1) length(2) host.
fn server_name(extension: &[u8]) -> Option<String> {
    let list_len = usize::from(be_u16(extension, 0)?);
    let list_end = 2_usize.checked_add(list_len)?.min(extension.len());
    let mut cursor = 2_usize;
    while cursor + 3 <= list_end {
        let name_type = *extension.get(cursor)?;
        let name_len = usize::from(be_u16(extension, cursor + 1)?);
        let name_start = cursor + 3;
        let name_end = name_start.checked_add(name_len)?;
        if name_end > list_end {
            return None;
        }
        if name_type == SNI_TYPE_HOST_NAME {
            let host = extension.get(name_start..name_end)?;
            let host = String::from_utf8_lossy(host).into_owned();
            return (!host.is_empty()).then_some(host);
        }
        cursor = name_end;
    }
    None
}

fn version_name(major: u8, minor: u8) -> Option<String> {
    let name = match (major, minor) {
        (3, 0) => "SSL 3.0",
        (3, 1) => "TLS 1.0",
        (3, 2) => "TLS 1.1",
        (3, 3) => "TLS 1.2+",
        (3, 4) => "TLS 1.3",
        _ => return None,
    };
    Some(name.to_owned())
}

fn be_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let pair = bytes.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([pair[0], pair[1]]))
}
