//! Reads the four `GetExtended*Table` variants into [`ConnectionOwner`] rows.
//!
//! Each table is a flexible-array struct: a `u32` entry count followed by that many
//! rows. Rust has no safe representation for that, so the row slice is reconstructed by
//! hand — always clamped to what the returned buffer can actually hold.

use std::{
    ffi::c_void,
    mem,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use windows::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR},
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
        MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID,
        MIB_UDP6TABLE_OWNER_PID, MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID,
        TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    },
    Networking::WinSock::{AF_INET, AF_INET6},
};

use crate::{
    attribution::{AttributionError, ConnectionOwner},
    model::TransportProtocol,
};

/// The table can grow between sizing it and reading it; retry a bounded number of times.
const MAX_ATTEMPTS: usize = 4;

pub fn tcp_v4() -> Result<Vec<ConnectionOwner>, AttributionError> {
    let buffer = query(|table, size| {
        // SAFETY: `table` is either null (sizing) or a buffer of `*size` bytes.
        unsafe {
            GetExtendedTcpTable(
                table,
                size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        }
    })?;
    // SAFETY: the call above filled `buffer` with a MIB_TCPTABLE_OWNER_PID.
    let rows: &[MIB_TCPROW_OWNER_PID] = unsafe { rows_of::<MIB_TCPTABLE_OWNER_PID, _>(&buffer) };
    Ok(rows
        .iter()
        .map(|row| ConnectionOwner {
            protocol: TransportProtocol::Tcp,
            local_ip: ipv4(row.dwLocalAddr),
            local_port: port(row.dwLocalPort),
            remote: peer(ipv4(row.dwRemoteAddr), port(row.dwRemotePort)),
            pid: row.dwOwningPid,
        })
        .collect())
}

pub fn tcp_v6() -> Result<Vec<ConnectionOwner>, AttributionError> {
    let buffer = query(|table, size| {
        // SAFETY: as above, for the IPv6 table.
        unsafe {
            GetExtendedTcpTable(
                table,
                size,
                false,
                AF_INET6.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        }
    })?;
    // SAFETY: the call above filled `buffer` with a MIB_TCP6TABLE_OWNER_PID.
    let rows: &[MIB_TCP6ROW_OWNER_PID] = unsafe { rows_of::<MIB_TCP6TABLE_OWNER_PID, _>(&buffer) };
    Ok(rows
        .iter()
        .map(|row| ConnectionOwner {
            protocol: TransportProtocol::Tcp,
            local_ip: ipv6(row.ucLocalAddr),
            local_port: port(row.dwLocalPort),
            remote: peer(ipv6(row.ucRemoteAddr), port(row.dwRemotePort)),
            pid: row.dwOwningPid,
        })
        .collect())
}

pub fn udp_v4() -> Result<Vec<ConnectionOwner>, AttributionError> {
    let buffer = query(|table, size| {
        // SAFETY: as above, for the UDP table.
        unsafe { GetExtendedUdpTable(table, size, false, AF_INET.0 as u32, UDP_TABLE_OWNER_PID, 0) }
    })?;
    // SAFETY: the call above filled `buffer` with a MIB_UDPTABLE_OWNER_PID.
    let rows: &[MIB_UDPROW_OWNER_PID] = unsafe { rows_of::<MIB_UDPTABLE_OWNER_PID, _>(&buffer) };
    Ok(rows
        .iter()
        .map(|row| ConnectionOwner {
            protocol: TransportProtocol::Udp,
            local_ip: ipv4(row.dwLocalAddr),
            local_port: port(row.dwLocalPort),
            // UDP is connectionless: the table never reports a peer.
            remote: None,
            pid: row.dwOwningPid,
        })
        .collect())
}

pub fn udp_v6() -> Result<Vec<ConnectionOwner>, AttributionError> {
    let buffer = query(|table, size| {
        // SAFETY: as above, for the IPv6 UDP table.
        unsafe {
            GetExtendedUdpTable(
                table,
                size,
                false,
                AF_INET6.0 as u32,
                UDP_TABLE_OWNER_PID,
                0,
            )
        }
    })?;
    // SAFETY: the call above filled `buffer` with a MIB_UDP6TABLE_OWNER_PID.
    let rows: &[MIB_UDP6ROW_OWNER_PID] = unsafe { rows_of::<MIB_UDP6TABLE_OWNER_PID, _>(&buffer) };
    Ok(rows
        .iter()
        .map(|row| ConnectionOwner {
            protocol: TransportProtocol::Udp,
            local_ip: ipv6(row.ucLocalAddr),
            local_port: port(row.dwLocalPort),
            remote: None,
            pid: row.dwOwningPid,
        })
        .collect())
}

/// Runs the two-step size-then-read dance the IP Helper tables require.
///
/// The buffer is `u32`-backed because every `MIB_*` row is `u32`-aligned and `Vec<u8>`
/// makes no alignment promise.
fn query(
    mut call: impl FnMut(Option<*mut c_void>, *mut u32) -> u32,
) -> Result<Vec<u32>, AttributionError> {
    for _ in 0..MAX_ATTEMPTS {
        let mut size = 0_u32;
        let status = call(None, &mut size);
        if status != ERROR_INSUFFICIENT_BUFFER.0 && status != NO_ERROR.0 {
            return Err(AttributionError::ConnectionTable(format!(
                "sizing the table failed with status {status}"
            )));
        }
        if size == 0 {
            return Ok(Vec::new());
        }

        let words = (size as usize).div_ceil(mem::size_of::<u32>());
        let mut buffer = vec![0_u32; words];
        let mut capacity = (words * mem::size_of::<u32>()) as u32;
        let status = call(Some(buffer.as_mut_ptr().cast::<c_void>()), &mut capacity);
        if status == NO_ERROR.0 {
            return Ok(buffer);
        }
        // Anything other than "it grew" is a real failure.
        if status != ERROR_INSUFFICIENT_BUFFER.0 {
            return Err(AttributionError::ConnectionTable(format!(
                "reading the table failed with status {status}"
            )));
        }
    }
    Err(AttributionError::ConnectionTable(
        "the connection table kept growing between reads".to_owned(),
    ))
}

/// Reinterprets a filled table buffer as its trailing row slice.
///
/// # Safety
/// `buffer` must hold the `Table` struct one of the `GetExtended*Table` calls wrote:
/// a leading `u32` entry count followed by `Row` values. The declared count is clamped
/// to the buffer's real capacity, so a bogus count cannot read out of bounds.
unsafe fn rows_of<Table, Row>(buffer: &[u32]) -> &[Row] {
    // A table with entries always includes its header word.
    if buffer.is_empty() {
        return &[];
    }
    let declared = *buffer.as_ptr() as usize;
    let offset = mem::size_of::<Table>().saturating_sub(mem::size_of::<Row>());
    let bytes = mem::size_of_val(buffer);
    let capacity = bytes.saturating_sub(offset) / mem::size_of::<Row>().max(1);
    let first = buffer.as_ptr().cast::<u8>().add(offset).cast::<Row>();
    std::slice::from_raw_parts(first, declared.min(capacity))
}

/// `dwLocalAddr`/`dwRemoteAddr` hold the address in network byte order, so the raw
/// memory bytes are already the octets.
fn ipv4(raw: u32) -> IpAddr {
    IpAddr::V4(Ipv4Addr::from(raw.to_ne_bytes()))
}

fn ipv6(raw: [u8; 16]) -> IpAddr {
    IpAddr::V6(Ipv6Addr::from(raw))
}

/// Ports are stored in network byte order in the low two bytes of a `DWORD`.
fn port(raw: u32) -> u16 {
    u16::from_be_bytes([(raw & 0xff) as u8, ((raw >> 8) & 0xff) as u8])
}

/// A listening TCP socket reports an all-zero peer; that is "no peer", not a real one.
fn peer(address: IpAddr, port: u16) -> Option<(IpAddr, u16)> {
    (!address.is_unspecified() || port != 0).then_some((address, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_network_order_ports() {
        // 0x01BB little-endian in the low word == port 443.
        assert_eq!(port(0x0000_bb01), 443);
        assert_eq!(port(0), 0);
    }

    #[test]
    fn decodes_network_order_ipv4_addresses() {
        let raw = u32::from_ne_bytes([192, 168, 1, 20]);
        assert_eq!(ipv4(raw).to_string(), "192.168.1.20");
    }

    #[test]
    fn treats_an_all_zero_tcp_peer_as_no_peer() {
        assert_eq!(peer(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0), None);
        assert!(peer(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 443).is_some());
    }

    #[test]
    fn row_slices_are_clamped_to_the_buffer() {
        // A header claiming 1000 rows in a buffer that holds one must yield one row.
        let mut buffer = vec![0_u32; 1 + 6];
        buffer[0] = 1000;
        // SAFETY: the buffer is laid out as MIB_TCPTABLE_OWNER_PID with room for one row.
        let rows: &[MIB_TCPROW_OWNER_PID] =
            unsafe { rows_of::<MIB_TCPTABLE_OWNER_PID, _>(&buffer) };
        assert_eq!(rows.len(), 1);
    }
}
