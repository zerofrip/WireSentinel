use crate::stub::parse_protocol;
use shared_types::ConnectionSnapshot;

#[cfg(windows)]
use windows::Win32::NetworkManagement::IpHelper::{MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID};

pub fn enumerate_tcp_connections() -> Vec<ConnectionSnapshot> {
    enumerate_tcp_connections_impl()
}

#[cfg(windows)]
fn enumerate_tcp_connections_impl() -> Vec<ConnectionSnapshot> {
    let mut connections = Vec::new();
    connections.extend(enumerate_tcp_v4());
    connections.extend(enumerate_tcp_v6());
    connections
}

#[cfg(windows)]
fn enumerate_tcp_v4() -> Vec<ConnectionSnapshot> {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::Foundation::NO_ERROR;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    };
    use windows::Win32::Networking::WinSock::AF_INET;

    let mut connections = Vec::new();
    let mut size = 0u32;
    unsafe {
        let _ = GetExtendedTcpTable(
            None,
            &mut size,
            BOOL(0),
            AF_INET.0.into(),
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
    }
    if size == 0 {
        return connections;
    }

    let mut buffer = vec![0u8; size as usize];
    let status = unsafe {
        GetExtendedTcpTable(
            Some(buffer.as_mut_ptr().cast()),
            &mut size,
            BOOL(0),
            AF_INET.0.into(),
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };
    if status != NO_ERROR.0 {
        return connections;
    }

    unsafe {
        let table = &*(buffer.as_ptr().cast::<MIB_TCPTABLE_OWNER_PID>());
        let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
        for row in rows {
            connections.push(row_to_snapshot_v4(row));
        }
    }
    connections
}

#[cfg(windows)]
fn enumerate_tcp_v6() -> Vec<ConnectionSnapshot> {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::Foundation::NO_ERROR;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6TABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    };
    use windows::Win32::Networking::WinSock::AF_INET6;

    let mut connections = Vec::new();
    let mut size = 0u32;
    unsafe {
        let _ = GetExtendedTcpTable(
            None,
            &mut size,
            BOOL(0),
            AF_INET6.0.into(),
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
    }
    if size == 0 {
        return connections;
    }

    let mut buffer = vec![0u8; size as usize];
    let status = unsafe {
        GetExtendedTcpTable(
            Some(buffer.as_mut_ptr().cast()),
            &mut size,
            BOOL(0),
            AF_INET6.0.into(),
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };
    if status != NO_ERROR.0 {
        return connections;
    }

    unsafe {
        let table = &*(buffer.as_ptr().cast::<MIB_TCP6TABLE_OWNER_PID>());
        let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
        for row in rows {
            connections.push(row_to_snapshot_v6(row));
        }
    }
    connections
}

#[cfg(windows)]
fn row_to_snapshot_v4(row: &MIB_TCPROW_OWNER_PID) -> ConnectionSnapshot {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let local = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::from(row.dwLocalAddr.to_be())),
        u16::from_be(row.dwLocalPort as u16),
    );
    let remote = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::from(row.dwRemoteAddr.to_be())),
        u16::from_be(row.dwRemotePort as u16),
    );
    ConnectionSnapshot {
        pid: row.dwOwningPid,
        app_id: None,
        exe_name: format!("pid:{}", row.dwOwningPid),
        protocol: parse_protocol(6),
        local_addr: local,
        remote_addr: remote,
        state: format!("state:{}", row.dwState),
        remote_domain: None,
        bytes_sent: 0,
        bytes_received: 0,
    }
}

#[cfg(windows)]
fn row_to_snapshot_v6(row: &MIB_TCP6ROW_OWNER_PID) -> ConnectionSnapshot {
    use std::net::{IpAddr, Ipv6Addr, SocketAddr};

    let local = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)),
        u16::from_be(row.dwLocalPort as u16),
    );
    let remote = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::from(row.ucRemoteAddr)),
        u16::from_be(row.dwRemotePort as u16),
    );
    ConnectionSnapshot {
        pid: row.dwOwningPid,
        app_id: None,
        exe_name: format!("pid:{}", row.dwOwningPid),
        protocol: parse_protocol(6),
        local_addr: local,
        remote_addr: remote,
        state: format!("state:{}", row.dwState),
        remote_domain: None,
        bytes_sent: 0,
        bytes_received: 0,
    }
}

#[cfg(not(windows))]
fn enumerate_tcp_connections_impl() -> Vec<ConnectionSnapshot> {
    Vec::new()
}
