//! UDP DNS proxy — evaluates filter policy before forwarding to upstream.

use crate::policy::{evaluate_domain, DomainDecision};
use async_trait::async_trait;
use filter_lists::FilterListProvider;
use shared_types::{DNSQueryLog, DnsBlockMode, DnsSettings, Result, WireSentinelError};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, watch};
use tracing::{debug, warn};
use uuid::Uuid;

#[async_trait]
pub trait UpstreamLookup: Send + Sync {
    async fn resolve_a(&self, name: &str) -> Result<Vec<String>>;
    fn provider_name(&self) -> String;
}

pub type DnsLogHandler = Arc<dyn Fn(DNSQueryLog) + Send + Sync>;

pub struct DnsProxyHandle {
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl DnsProxyHandle {
    pub fn stop(self) {
        let _ = self.shutdown.send(true);
        self.task.abort();
    }
}

pub struct ProxyContext {
    pub settings: DnsSettings,
    pub filter: Option<Arc<dyn FilterListProvider>>,
    pub upstream: Arc<dyn UpstreamLookup>,
    pub log_tx: broadcast::Sender<DNSQueryLog>,
    pub log_handler: Option<DnsLogHandler>,
}

pub fn spawn_proxy(ctx: Arc<ProxyContext>) -> Result<DnsProxyHandle> {
    let listen = ctx
        .settings
        .listen_addr
        .parse::<SocketAddr>()
        .map_err(|e| WireSentinelError::Dns(format!("invalid listen_addr: {e}")))?;
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(run_proxy(listen, ctx, shutdown_rx));
    Ok(DnsProxyHandle {
        shutdown: shutdown_tx,
        task,
    })
}

async fn run_proxy(
    listen: SocketAddr,
    ctx: Arc<ProxyContext>,
    mut shutdown: watch::Receiver<bool>,
) {
    let socket = match UdpSocket::bind(listen).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            warn!(error = %e, %listen, "DNS proxy failed to bind");
            return;
        }
    };
    debug!(%listen, "DNS UDP proxy listening");

    let mut buf = [0u8; 512];
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
            recv = socket.recv_from(&mut buf) => {
                let Ok((len, peer)) = recv else { continue };
                if len < 12 {
                    continue;
                }
                let query = buf[..len].to_vec();
                let Some((qname, qtype)) = parse_question(&query) else {
                    continue;
                };

                let ctx = Arc::clone(&ctx);
                let sock = Arc::clone(&socket);
                tokio::spawn(async move {
                    let settings = ctx.settings.clone();
                    let filter = ctx.filter.clone();
                    let upstream = Arc::clone(&ctx.upstream);
                    let log_tx = ctx.log_tx.clone();
                    let log_handler = ctx.log_handler.clone();
                    let block_mode = settings.dns_block_mode;
                    let filter_mode = settings.filter_mode;

                    let decision = evaluate_domain(
                        &qname,
                        filter_mode,
                        filter.as_deref(),
                    );

                    let correlation_id = Uuid::new_v4();
                    let response = if decision == DomainDecision::Block {
                        let log = DNSQueryLog {
                            id: Uuid::new_v4(),
                            timestamp: chrono::Utc::now(),
                            app_id: None,
                            pid: None,
                            qname: qname.clone(),
                            qtype: qtype.clone(),
                            upstream: settings.provider.clone(),
                            blocked: true,
                            latency_ms: 0,
                            answers: vec![],
                            response: Some(format!("blocked:{block_mode:?}")),
                            correlation_id: Some(correlation_id),
                        };
                        let _ = log_tx.send(log.clone());
                        if let Some(ref handler) = log_handler {
                            handler(log);
                        }
                        build_blocked_response(&query, block_mode)
                    } else {
                        let start = std::time::Instant::now();
                        match upstream.resolve_a(&qname).await {
                            Ok(answers) => {
                                let latency = start.elapsed().as_millis() as u64;
                                let log = DNSQueryLog {
                                    id: Uuid::new_v4(),
                                    timestamp: chrono::Utc::now(),
                                    app_id: None,
                                    pid: None,
                                    qname: qname.clone(),
                                    qtype: qtype.clone(),
                                    upstream: upstream.provider_name().to_string(),
                                    blocked: false,
                                    latency_ms: latency,
                                    answers: answers.clone(),
                                    response: None,
                                    correlation_id: Some(correlation_id),
                                };
                                let _ = log_tx.send(log.clone());
                                if let Some(ref handler) = log_handler {
                                    handler(log);
                                }
                                build_answer_response(&query, &answers)
                            }
                            Err(e) => {
                                warn!(qname, error = %e, "upstream DNS resolve failed");
                                build_servfail_response(&query)
                            }
                        }
                    };

                    if let Some(resp) = response {
                        let _ = sock.send_to(&resp, peer).await;
                    }
                });
            }
        }
    }
    debug!("DNS UDP proxy stopped");
}

fn parse_question(packet: &[u8]) -> Option<(String, String)> {
    if packet.len() < 12 {
        return None;
    }
    let qd_count = u16::from_be_bytes([packet[4], packet[5]]);
    if qd_count == 0 {
        return None;
    }
    let mut offset = 12usize;
    let name = read_name(packet, &mut offset)?;
    if offset + 4 > packet.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
    let qtype_str = match qtype {
        1 => "A",
        28 => "AAAA",
        5 => "CNAME",
        _ => "OTHER",
    }
    .to_string();
    Some((name, qtype_str))
}

fn read_name(packet: &[u8], offset: &mut usize) -> Option<String> {
    let mut labels = Vec::new();
    let mut jumped = false;
    let mut jump_offset = 0usize;
    loop {
        if *offset >= packet.len() {
            return None;
        }
        let len = packet[*offset] as usize;
        if len == 0 {
            *offset += 1;
            break;
        }
        if len & 0xC0 == 0xC0 {
            if *offset + 1 >= packet.len() {
                return None;
            }
            let ptr = u16::from_be_bytes([packet[*offset] & 0x3F, packet[*offset + 1]]) as usize;
            if !jumped {
                jump_offset = *offset + 2;
            }
            *offset = ptr;
            jumped = true;
            continue;
        }
        *offset += 1;
        if *offset + len > packet.len() {
            return None;
        }
        labels.push(String::from_utf8_lossy(&packet[*offset..*offset + len]).to_string());
        *offset += len;
    }
    if jumped {
        *offset = jump_offset;
    }
    Some(labels.join("."))
}

fn build_blocked_response(query: &[u8], mode: DnsBlockMode) -> Option<Vec<u8>> {
    match mode {
        DnsBlockMode::Nxdomain => build_nxdomain_response(query),
        DnsBlockMode::Null => build_null_a_response(query),
    }
}

fn build_nxdomain_response(query: &[u8]) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let mut resp = query.to_vec();
    resp[2] = 0x81;
    resp[3] = 0x83;
    resp[6] = 0;
    resp[7] = 0;
    resp[8] = 0;
    resp[9] = 0;
    resp[10] = 0;
    resp[11] = 0;
    Some(resp)
}

fn build_servfail_response(query: &[u8]) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let mut resp = query.to_vec();
    resp[2] = 0x81;
    resp[3] = 0x82;
    Some(resp)
}

fn build_null_a_response(query: &[u8]) -> Option<Vec<u8>> {
    let q_end = question_end(query)?;
    let mut resp = Vec::with_capacity(q_end + 16);
    resp.extend_from_slice(&query[..2]);
    resp.push(0x81);
    resp.push(0x80);
    resp.extend_from_slice(&query[4..6]);
    resp.extend_from_slice(&[0, 1]);
    resp.extend_from_slice(&[0, 0, 0, 0]);
    resp.extend_from_slice(&query[12..q_end]);
    resp.extend_from_slice(&[0xC0, 0x0C]);
    resp.extend_from_slice(&[0, 1, 0, 1]);
    resp.extend_from_slice(&[0, 0, 1, 0x2C]);
    resp.extend_from_slice(&[0, 4]);
    resp.extend_from_slice(&[0, 0, 0, 0]);
    Some(resp)
}

fn build_answer_response(query: &[u8], answers: &[String]) -> Option<Vec<u8>> {
    let q_end = question_end(query)?;
    let mut resp = Vec::new();
    resp.extend_from_slice(&query[..2]);
    resp.push(0x81);
    resp.push(0x80);
    resp.extend_from_slice(&query[4..6]);
    let ancount = answers.len().min(8) as u16;
    resp.extend_from_slice(&ancount.to_be_bytes());
    resp.extend_from_slice(&[0, 0, 0, 0]);
    resp.extend_from_slice(&query[12..q_end]);

    for ip in answers.iter().take(8) {
        if ip.parse::<std::net::Ipv4Addr>().is_err() {
            continue;
        }
        let octets: [u8; 4] = ip
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect::<Vec<u8>>()
            .try_into()
            .ok()?;
        resp.extend_from_slice(&[0xC0, 0x0C]);
        resp.extend_from_slice(&[0, 1, 0, 1]);
        resp.extend_from_slice(&[0, 0, 1, 0x2C]);
        resp.extend_from_slice(&[0, 4]);
        resp.extend_from_slice(&octets);
    }
    Some(resp)
}

fn question_end(packet: &[u8]) -> Option<usize> {
    let mut offset = 12usize;
    loop {
        if offset >= packet.len() {
            return None;
        }
        let len = packet[offset] as usize;
        if len == 0 {
            offset += 1;
            break;
        }
        if len & 0xC0 == 0xC0 {
            offset += 2;
            break;
        }
        offset += 1 + len;
    }
    if offset + 4 > packet.len() {
        return None;
    }
    Some(offset + 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::DnsBlockMode;

    fn sample_query() -> Vec<u8> {
        vec![
            0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, b'e',
            b'x', b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00, 0x01, 0x00,
            0x01,
        ]
    }

    #[test]
    fn blocked_nxdomain_sets_rcode() {
        let resp = build_blocked_response(&sample_query(), DnsBlockMode::Nxdomain).unwrap();
        assert_eq!(resp[3] & 0x0F, 3);
    }

    #[test]
    fn blocked_null_returns_zero_a() {
        let resp = build_blocked_response(&sample_query(), DnsBlockMode::Null).unwrap();
        assert!(resp.ends_with(&[0, 0, 0, 0]));
    }
}
