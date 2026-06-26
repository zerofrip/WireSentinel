//! Connection eligibility filters (skip invalid PIDs, LISTEN sockets, etc.).

use shared_types::{ConnectionSnapshot, Protocol};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// MIB_TCP_STATE_ESTAB — only established TCP rows are processed from iphlpapi polls.
#[cfg(windows)]
pub const MIB_TCP_STATE_ESTAB: u32 = 5;

#[cfg(windows)]
const MIB_TCP_STATE_LISTEN: u32 = 2;

pub fn is_valid_pid(pid: u32) -> bool {
    pid > 0
}

pub fn is_processable_connection(conn: &ConnectionSnapshot) -> bool {
    if !is_valid_pid(conn.pid) {
        return false;
    }
    if conn.remote_addr.port() == 0 {
        return false;
    }
    if is_unspecified_remote(&conn.remote_addr.ip()) {
        return false;
    }
    if conn.protocol == Protocol::Tcp {
        if let Some(state) = parse_tcp_state(&conn.state) {
            #[cfg(windows)]
            {
                if state == MIB_TCP_STATE_LISTEN {
                    return false;
                }
                return state == MIB_TCP_STATE_ESTAB;
            }
            #[cfg(not(windows))]
            {
                let _ = state;
            }
        }
    }
    true
}

fn is_unspecified_remote(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => *v4 == Ipv4Addr::UNSPECIFIED,
        IpAddr::V6(v6) => *v6 == Ipv6Addr::UNSPECIFIED,
    }
}

fn parse_tcp_state(state: &str) -> Option<u32> {
    state.strip_prefix("state:").and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Protocol;
    use std::net::SocketAddr;

    #[test]
    fn rejects_pid_zero() {
        let conn = ConnectionSnapshot {
            pid: 0,
            app_id: None,
            exe_name: "pid:0".into(),
            protocol: Protocol::Tcp,
            local_addr: "127.0.0.1:1234".parse().unwrap(),
            remote_addr: "93.184.216.34:443".parse().unwrap(),
            state: "state:5".into(),
            remote_domain: None,
            bytes_sent: 0,
            bytes_received: 0,
        };
        assert!(!is_processable_connection(&conn));
    }

    #[test]
    fn accepts_packet_new_state() {
        let conn = ConnectionSnapshot {
            pid: 1234,
            app_id: None,
            exe_name: "pid:1234".into(),
            protocol: Protocol::Tcp,
            local_addr: "127.0.0.1:1234".parse().unwrap(),
            remote_addr: "93.184.216.34:443".parse().unwrap(),
            state: "packet:new".into(),
            remote_domain: None,
            bytes_sent: 0,
            bytes_received: 0,
        };
        assert!(is_processable_connection(&conn));
    }
}
