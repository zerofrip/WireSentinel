use async_trait::async_trait;
use shared_types::{DnsTransport, ProviderHealth, Result, WireSentinelError};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;
use tracing::debug;

use crate::provider::DnsProvider;

/// DNS-over-TLS provider (RFC 7858 wire format over TLS).
pub struct DotProvider {
    host: String,
    port: u16,
    connector: TlsConnector,
}

impl DotProvider {
    pub fn new(host: &str, port: u16) -> Self {
        let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        Self {
            host: host.to_string(),
            port,
            connector: TlsConnector::from(Arc::new(config)),
        }
    }
}

#[async_trait]
impl DnsProvider for DotProvider {
    fn name(&self) -> &str {
        &self.host
    }

    fn transport(&self) -> DnsTransport {
        DnsTransport::Dot
    }

    async fn resolve(&self, qname: &str, qtype: &str) -> Result<(Vec<String>, u64)> {
        let start = Instant::now();
        let query = build_query(qname, qtype);
        let response = self.exchange(&query).await?;
        let answers = parse_a_records(&response);
        let latency = start.elapsed().as_millis() as u64;
        debug!(name = qname, ?answers, latency_ms = latency, "DoT resolved");
        Ok((answers, latency))
    }

    async fn health_check(&self) -> ProviderHealth {
        match self.resolve("example.com", "A").await {
            Ok((answers, latency)) if !answers.is_empty() => ProviderHealth {
                available: true,
                latency_ms: Some(latency),
                message: None,
            },
            Ok(_) => ProviderHealth {
                available: false,
                latency_ms: None,
                message: Some("empty response".into()),
            },
            Err(e) => ProviderHealth {
                available: false,
                latency_ms: None,
                message: Some(e.to_string()),
            },
        }
    }
}

impl DotProvider {
    async fn exchange(&self, query: &[u8]) -> Result<Vec<u8>> {
        let addr = format!("{}:{}", self.host, self.port);
        let tcp = TcpStream::connect(&addr)
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoT connect failed: {e}")))?;

        let server_name = ServerName::try_from(self.host.clone())
            .map_err(|_| WireSentinelError::Dns("invalid DoT server name".into()))?;
        let mut tls = self
            .connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoT TLS handshake failed: {e}")))?;

        let len = (query.len() as u16).to_be_bytes();
        tls.write_all(&len)
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoT write failed: {e}")))?;
        tls.write_all(query)
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoT write failed: {e}")))?;

        let mut len_buf = [0u8; 2];
        tls.read_exact(&mut len_buf)
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoT read failed: {e}")))?;
        let resp_len = u16::from_be_bytes(len_buf) as usize;
        if resp_len == 0 {
            return Err(WireSentinelError::Dns("DoT invalid response length".into()));
        }
        let mut resp = vec![0u8; resp_len];
        tls.read_exact(&mut resp)
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoT read failed: {e}")))?;
        Ok(resp)
    }
}

fn qtype_code(qtype: &str) -> u16 {
    match qtype.to_uppercase().as_str() {
        "AAAA" => 28,
        "CNAME" => 5,
        _ => 1,
    }
}

fn build_query(name: &str, qtype: &str) -> Vec<u8> {
    let mut packet = vec![
        0xAA, 0xBB, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    for label in name.trim_end_matches('.').split('.') {
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0);
    packet.extend_from_slice(&qtype_code(qtype).to_be_bytes());
    packet.extend_from_slice(&[0, 1]);
    packet
}

fn parse_a_records(packet: &[u8]) -> Vec<String> {
    if packet.len() < 12 {
        return Vec::new();
    }
    let ancount = u16::from_be_bytes([packet[6], packet[7]]) as usize;
    let mut offset = 12usize;
    if !skip_name(packet, &mut offset) {
        return Vec::new();
    }
    if offset + 4 > packet.len() {
        return Vec::new();
    }
    offset += 4;

    let mut answers = Vec::new();
    for _ in 0..ancount {
        if !skip_name(packet, &mut offset) {
            break;
        }
        if offset + 10 > packet.len() {
            break;
        }
        let rtype = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
        let rdlength = u16::from_be_bytes([packet[offset + 8], packet[offset + 9]]) as usize;
        offset += 10;
        if offset + rdlength > packet.len() {
            break;
        }
        if rtype == 1 && rdlength == 4 {
            let ip = format!(
                "{}.{}.{}.{}",
                packet[offset],
                packet[offset + 1],
                packet[offset + 2],
                packet[offset + 3]
            );
            answers.push(ip);
        }
        offset += rdlength;
    }
    answers
}

fn skip_name(packet: &[u8], offset: &mut usize) -> bool {
    loop {
        if *offset >= packet.len() {
            return false;
        }
        let len = packet[*offset] as usize;
        if len == 0 {
            *offset += 1;
            return true;
        }
        if len & 0xC0 == 0xC0 {
            if *offset + 1 >= packet.len() {
                return false;
            }
            *offset += 2;
            return true;
        }
        *offset += 1 + len;
    }
}
