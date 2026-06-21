//! TCP session termination — Windows uses SetTcpEntry (IPv4).

use async_trait::async_trait;
use shared_types::{Result, TcpConnectionSnapshot};

#[async_trait]
pub trait TcpSessionTerminator: Send + Sync {
    fn enumerate(&self) -> Vec<TcpConnectionSnapshot>;
    fn terminate(&self, conn: &TcpConnectionSnapshot) -> Result<()>;
}

pub fn default_terminator() -> Box<dyn TcpSessionTerminator> {
    #[cfg(windows)]
    {
        Box::new(windows::WindowsTcpTerminator)
    }
    #[cfg(not(windows))]
    {
        Box::new(stub::StubTcpTerminator)
    }
}

#[cfg(windows)]
mod windows {
    use super::TcpSessionTerminator;
    use shared_types::{Result, TcpConnectionSnapshot, WireSentinelError};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use traffic_monitor::windows::enumerate_tcp_connections;
    use windows::Win32::NetworkManagement::IpHelper::{
        SetTcpEntry, MIB_TCPROW, MIB_TCP_STATE_DELETE_TCB,
    };

    pub struct WindowsTcpTerminator;

    impl WindowsTcpTerminator {
        fn terminate_v4(conn: &TcpConnectionSnapshot) -> Result<()> {
            let local = conn.local_addr;
            let remote = conn.remote_addr;
            let (local_ip, local_port) = match local.ip() {
                IpAddr::V4(v4) => (v4, local.port()),
                _ => return Ok(()),
            };
            let (remote_ip, remote_port) = match remote.ip() {
                IpAddr::V4(v4) => (v4, remote.port()),
                _ => return Ok(()),
            };

            let mut row = MIB_TCPROW {
                dwState: MIB_TCP_STATE_DELETE_TCB.0,
                dwLocalAddr: u32::from(local_ip).to_be(),
                dwLocalPort: (local_port as u32).to_be(),
                dwRemoteAddr: u32::from(remote_ip).to_be(),
                dwRemotePort: (remote_port as u32).to_be(),
                ..Default::default()
            };

            unsafe {
                SetTcpEntry(&mut row).map_err(|e| {
                    WireSentinelError::Internal(format!("SetTcpEntry v4: {e}"))
                })?;
            }
            Ok(())
        }
    }

    impl TcpSessionTerminator for WindowsTcpTerminator {
        fn enumerate(&self) -> Vec<TcpConnectionSnapshot> {
            enumerate_tcp_connections()
                .into_iter()
                .map(TcpConnectionSnapshot::from)
                .collect()
        }

        fn terminate(&self, conn: &TcpConnectionSnapshot) -> Result<()> {
            match (conn.local_addr, conn.remote_addr) {
                (SocketAddr::V4(_), SocketAddr::V4(_)) => Self::terminate_v4(conn),
                _ => Ok(()),
            }
        }
    }
}

#[cfg(not(windows))]
mod stub {
    use super::TcpSessionTerminator;
    use shared_types::{Result, TcpConnectionSnapshot};

    pub struct StubTcpTerminator;

    impl TcpSessionTerminator for StubTcpTerminator {
        fn enumerate(&self) -> Vec<TcpConnectionSnapshot> {
            Vec::new()
        }

        fn terminate(&self, _conn: &TcpConnectionSnapshot) -> Result<()> {
            Ok(())
        }
    }
}
