use shared_types::{HandshakeProxySettings, ProxyType, Result, WireSentinelError};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::task::JoinHandle;
use tracing::{debug, warn};

const SOCKS5_VERSION: u8 = 0x05;
const SOCKS5_AUTH_NONE: u8 = 0x00;
const SOCKS5_AUTH_USERPASS: u8 = 0x02;
const SOCKS5_CMD_UDP_ASSOCIATE: u8 = 0x03;
const SOCKS5_ATYP_IPV4: u8 = 0x01;
const SOCKS5_ATYP_DOMAIN: u8 = 0x03;
const SOCKS5_ATYP_IPV6: u8 = 0x04;

/// Active SOCKS5 UDP relay for handshake phase.
pub struct PreparedHandshake {
    pub relay_endpoint: String,
    _relay_task: JoinHandle<()>,
}

pub struct Socks5HandshakeBackend {
    settings: HandshakeProxySettings,
}

impl Socks5HandshakeBackend {
    pub fn new(settings: HandshakeProxySettings) -> Self {
        Self { settings }
    }

    pub fn from_profile(profile: &shared_types::VPNProfile) -> Option<Self> {
        profile
            .handshake_proxy
            .as_ref()
            .filter(|s| s.enabled && !s.host.is_empty())
            .cloned()
            .map(Self::new)
    }

    /// Establish SOCKS5 UDP associate relay for WireGuard endpoint `host:port`.
    pub async fn prepare_handshake_endpoint(&self, wg_endpoint: &str) -> Result<PreparedHandshake> {
        if self.settings.proxy_type != ProxyType::Socks5 {
            return Err(WireSentinelError::Vpn(
                "handshake proxy: only SOCKS5 is supported".into(),
            ));
        }

        let (target_host, target_port) = parse_host_port(wg_endpoint)?;
        let proxy_addr = format!("{}:{}", self.settings.host, self.settings.port);
        let mut stream = TcpStream::connect(&proxy_addr)
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("socks5 connect {proxy_addr}: {e}")))?;

        self.socks5_auth(&mut stream).await?;
        let bind_addr = self
            .socks5_udp_associate(&mut stream, &target_host, target_port)
            .await?;

        let relay = UdpSocket::bind("127.0.0.1:0")
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("udp relay bind: {e}")))?;
        let relay_local = relay
            .local_addr()
            .map_err(|e| WireSentinelError::Vpn(e.to_string()))?;

        let proxy_host = self.settings.host.clone();
        let proxy_port = self.settings.port;
        let username = self.settings.username.clone();
        let password = self.settings.password.clone();

        let relay_task = tokio::spawn(async move {
            if let Err(e) = udp_relay_loop(
                relay,
                proxy_host,
                proxy_port,
                username,
                password,
                bind_addr,
                target_host,
                target_port,
            )
            .await
            {
                warn!(error = %e, "handshake udp relay stopped");
            }
        });

        let relay_endpoint = format!("127.0.0.1:{}", relay_local.port());
        debug!(%relay_endpoint, %wg_endpoint, "socks5 handshake relay ready");

        Ok(PreparedHandshake {
            relay_endpoint,
            _relay_task: relay_task,
        })
    }
}

impl Socks5HandshakeBackend {
    async fn socks5_auth(&self, stream: &mut TcpStream) -> Result<()> {
        let methods = if self.settings.username.is_some() {
            vec![SOCKS5_AUTH_NONE, SOCKS5_AUTH_USERPASS]
        } else {
            vec![SOCKS5_AUTH_NONE]
        };
        let mut greeting = vec![SOCKS5_VERSION, methods.len() as u8];
        greeting.extend(methods);
        stream
            .write_all(&greeting)
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("socks5 greeting: {e}")))?;

        let mut resp = [0u8; 2];
        stream
            .read_exact(&mut resp)
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("socks5 greeting resp: {e}")))?;
        if resp[0] != SOCKS5_VERSION {
            return Err(WireSentinelError::Vpn("invalid socks5 version".into()));
        }

        match resp[1] {
            SOCKS5_AUTH_NONE => Ok(()),
            SOCKS5_AUTH_USERPASS => {
                let user = self.settings.username.as_deref().unwrap_or("");
                let pass = self.settings.password.as_deref().unwrap_or("");
                let mut req = vec![0x01, user.len() as u8];
                req.extend(user.as_bytes());
                req.push(pass.len() as u8);
                req.extend(pass.as_bytes());
                stream
                    .write_all(&req)
                    .await
                    .map_err(|e| WireSentinelError::Vpn(format!("socks5 auth: {e}")))?;
                let mut auth_resp = [0u8; 2];
                stream
                    .read_exact(&mut auth_resp)
                    .await
                    .map_err(|e| WireSentinelError::Vpn(format!("socks5 auth resp: {e}")))?;
                if auth_resp[1] != 0 {
                    return Err(WireSentinelError::Vpn("socks5 auth failed".into()));
                }
                Ok(())
            }
            other => Err(WireSentinelError::Vpn(format!(
                "unsupported socks5 auth method {other}"
            ))),
        }
    }

    async fn socks5_udp_associate(
        &self,
        stream: &mut TcpStream,
        target_host: &str,
        target_port: u16,
    ) -> Result<SocketAddr> {
        let mut req = vec![SOCKS5_VERSION, SOCKS5_CMD_UDP_ASSOCIATE, 0x00];
        encode_address(&mut req, target_host)?;
        req.extend_from_slice(&target_port.to_be_bytes());

        stream
            .write_all(&req)
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("socks5 udp associate req: {e}")))?;

        let mut head = [0u8; 4];
        stream
            .read_exact(&mut head)
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("socks5 udp associate resp: {e}")))?;
        if head[1] != 0 {
            return Err(WireSentinelError::Vpn(format!(
                "socks5 udp associate failed code {}",
                head[1]
            )));
        }

        parse_bound_address(stream, head[3]).await
    }
}

async fn parse_bound_address(stream: &mut TcpStream, atyp: u8) -> Result<SocketAddr> {
    match atyp {
        SOCKS5_ATYP_IPV4 => {
            let mut buf = [0u8; 4 + 2];
            stream
                .read_exact(&mut buf)
                .await
                .map_err(|e| WireSentinelError::Vpn(format!("socks5 bound ipv4: {e}")))?;
            let ip = std::net::Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
            let port = u16::from_be_bytes([buf[4], buf[5]]);
            Ok(SocketAddr::new(ip.into(), port))
        }
        SOCKS5_ATYP_IPV6 => {
            let mut buf = [0u8; 16 + 2];
            stream
                .read_exact(&mut buf)
                .await
                .map_err(|e| WireSentinelError::Vpn(format!("socks5 bound ipv6: {e}")))?;
            let ip = std::net::Ipv6Addr::from(<[u8; 16]>::try_from(&buf[..16]).unwrap());
            let port = u16::from_be_bytes([buf[16], buf[17]]);
            Ok(SocketAddr::new(ip.into(), port))
        }
        SOCKS5_ATYP_DOMAIN => {
            let mut len = [0u8; 1];
            stream
                .read_exact(&mut len)
                .await
                .map_err(|e| WireSentinelError::Vpn(format!("socks5 bound domain len: {e}")))?;
            let mut domain = vec![0u8; len[0] as usize + 2];
            stream
                .read_exact(&mut domain)
                .await
                .map_err(|e| WireSentinelError::Vpn(format!("socks5 bound domain: {e}")))?;
            let port = u16::from_be_bytes([domain[domain.len() - 2], domain[domain.len() - 1]]);
            let host = String::from_utf8_lossy(&domain[..domain.len() - 2]).into_owned();
            let ip: std::net::IpAddr = tokio::net::lookup_host(format!("{host}:{port}"))
                .await
                .map_err(|e| WireSentinelError::Vpn(format!("resolve bound addr: {e}")))?
                .next()
                .ok_or_else(|| WireSentinelError::Vpn("resolve bound addr empty".into()))?
                .ip();
            Ok(SocketAddr::new(ip, port))
        }
        other => Err(WireSentinelError::Vpn(format!(
            "unknown socks5 atyp {other}"
        ))),
    }
}

fn encode_address(buf: &mut Vec<u8>, host: &str) -> Result<()> {
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        buf.push(SOCKS5_ATYP_IPV4);
        buf.extend(ip.octets());
        return Ok(());
    }
    if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
        buf.push(SOCKS5_ATYP_IPV6);
        buf.extend(ip.octets());
        return Ok(());
    }
    if host.len() > 255 {
        return Err(WireSentinelError::Vpn("domain too long".into()));
    }
    buf.push(SOCKS5_ATYP_DOMAIN);
    buf.push(host.len() as u8);
    buf.extend(host.as_bytes());
    Ok(())
}

fn parse_host_port(endpoint: &str) -> Result<(String, u16)> {
    if let Ok(addr) = endpoint.parse::<SocketAddr>() {
        return Ok((addr.ip().to_string(), addr.port()));
    }
    let (host, port) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| WireSentinelError::Vpn(format!("invalid endpoint: {endpoint}")))?;
    let port: u16 = port
        .parse()
        .map_err(|_| WireSentinelError::Vpn(format!("invalid port in {endpoint}")))?;
    Ok((host.to_string(), port))
}

#[allow(clippy::too_many_arguments)]
async fn udp_relay_loop(
    relay: UdpSocket,
    proxy_host: String,
    proxy_port: u16,
    username: Option<String>,
    password: Option<String>,
    _bind_addr: SocketAddr,
    target_host: String,
    target_port: u16,
) -> Result<()> {
    let proxy_addr = format!("{proxy_host}:{proxy_port}");
    let mut control = TcpStream::connect(&proxy_addr)
        .await
        .map_err(|e| WireSentinelError::Vpn(format!("relay control connect: {e}")))?;

    let backend = Socks5HandshakeBackend {
        settings: HandshakeProxySettings {
            enabled: true,
            proxy_type: ProxyType::Socks5,
            host: proxy_host,
            port: proxy_port,
            username,
            password,
        },
    };
    backend.socks5_auth(&mut control).await?;
    let socks_udp = backend
        .socks5_udp_associate(&mut control, &target_host, target_port)
        .await?;

    let mut buf = [0u8; 65535];
    loop {
        let (len, peer) = relay
            .recv_from(&mut buf)
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("relay recv: {e}")))?;

        let mut packet = Vec::with_capacity(len + 64);
        packet.extend_from_slice(&[0, 0, 0]);
        encode_address(&mut packet, &target_host)?;
        packet.extend_from_slice(&target_port.to_be_bytes());
        packet.extend_from_slice(&buf[..len]);

        let socks = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("relay socks udp: {e}")))?;
        socks
            .send_to(&packet, socks_udp)
            .await
            .map_err(|e| WireSentinelError::Vpn(format!("relay send: {e}")))?;

        if let Ok(Ok((resp_len, _))) =
            tokio::time::timeout(std::time::Duration::from_secs(2), socks.recv_from(&mut buf)).await
        {
            if resp_len > 0 {
                if let Ok(header_end) = packet_header_end(&buf[..resp_len]) {
                    let _ = relay.send_to(&buf[header_end..resp_len], peer).await;
                }
            }
        }
    }
}

fn packet_header_end(data: &[u8]) -> Result<usize> {
    if data.len() < 4 {
        return Err(WireSentinelError::Vpn("short socks udp packet".into()));
    }
    let atyp = data[3];
    let mut idx = 4;
    match atyp {
        SOCKS5_ATYP_IPV4 => idx += 4,
        SOCKS5_ATYP_IPV6 => idx += 16,
        SOCKS5_ATYP_DOMAIN => {
            if data.len() <= idx {
                return Err(WireSentinelError::Vpn("short domain header".into()));
            }
            idx += 1 + data[idx] as usize;
        }
        other => return Err(WireSentinelError::Vpn(format!("bad atyp {other}"))),
    }
    idx += 2;
    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_endpoint_ipv4() {
        let (h, p) = parse_host_port("1.2.3.4:51820").unwrap();
        assert_eq!(h, "1.2.3.4");
        assert_eq!(p, 51820);
    }

    #[test]
    fn parse_endpoint_domain() {
        let (h, p) = parse_host_port("vpn.example.com:443").unwrap();
        assert_eq!(h, "vpn.example.com");
        assert_eq!(p, 443);
    }
}
