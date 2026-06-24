//! Tor control port polling for bootstrap progress and circuit count.

use shared_types::{Result, WireSentinelError};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::sleep;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TorMetrics {
    pub bootstrap_progress: u8,
    pub circuit_count: u32,
}

pub fn control_port_for_socks(socks_port: u16) -> u16 {
    socks_port.saturating_add(1)
}

pub async fn poll_tor_metrics(control_port: u16, timeout: Duration) -> Result<TorMetrics> {
    let deadline = Instant::now() + timeout;
    let mut last = TorMetrics {
        bootstrap_progress: 0,
        circuit_count: 0,
    };

    while Instant::now() < deadline {
        match query_tor_metrics(control_port).await {
            Ok(metrics) => {
                last = metrics;
                if metrics.bootstrap_progress >= 100 {
                    return Ok(metrics);
                }
            }
            Err(_) => {}
        }
        sleep(Duration::from_millis(500)).await;
    }

    if last.bootstrap_progress > 0 {
        Ok(last)
    } else {
        Err(WireSentinelError::Other(format!(
            "tor control port {control_port} did not report bootstrap progress within {timeout:?}"
        )))
    }
}

async fn query_tor_metrics(control_port: u16) -> Result<TorMetrics> {
    let addr = format!("127.0.0.1:{control_port}");
    let stream = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(&addr))
        .await
        .map_err(|_| WireSentinelError::Other(format!("tor control connect timeout: {addr}")))?
        .map_err(|e| WireSentinelError::Other(format!("tor control connect {addr}: {e}")))?;

    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    let _ = lines.next_line().await;

    writer
        .write_all(b"AUTHENTICATE \"\"\r\n")
        .await
        .map_err(|e| WireSentinelError::Other(e.to_string()))?;
    read_until_ok(&mut lines).await?;

    writer
        .write_all(b"GETINFO status/progress\r\n")
        .await
        .map_err(|e| WireSentinelError::Other(e.to_string()))?;
    let progress_body = read_response_body(&mut lines).await?;
    let bootstrap_progress = parse_progress(&progress_body);

    writer
        .write_all(b"GETINFO circuit-established\r\n")
        .await
        .map_err(|e| WireSentinelError::Other(e.to_string()))?;
    let circuit_body = read_response_body(&mut lines).await?;
    let circuit_count = parse_circuit_count(&circuit_body);

    Ok(TorMetrics {
        bootstrap_progress,
        circuit_count,
    })
}

async fn read_until_ok(lines: &mut tokio::io::Lines<BufReader<tokio::net::tcp::OwnedReadHalf>>) -> Result<()> {
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| WireSentinelError::Other(e.to_string()))?
    {
        if line.starts_with("250 ") {
            return Ok(());
        }
        if line.starts_with("5") {
            return Err(WireSentinelError::Other(format!("tor control error: {line}")));
        }
    }
    Err(WireSentinelError::Other("tor control closed".into()))
}

async fn read_response_body(
    lines: &mut tokio::io::Lines<BufReader<tokio::net::tcp::OwnedReadHalf>>,
) -> Result<String> {
    let mut body = String::new();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| WireSentinelError::Other(e.to_string()))?
    {
        if line.starts_with("250 ") {
            break;
        }
        if line.starts_with("250-") {
            body.push_str(line.trim_start_matches("250-"));
            body.push('\n');
        } else if line.starts_with("5") {
            return Err(WireSentinelError::Other(format!("tor control error: {line}")));
        }
    }
    Ok(body)
}

fn parse_progress(body: &str) -> u8 {
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("PROGRESS=") {
            if let Ok(n) = rest.parse::<u8>() {
                return n.min(100);
            }
        }
    }
    0
}

fn parse_circuit_count(body: &str) -> u32 {
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("circuit-established=") {
            if rest == "1" || rest.eq_ignore_ascii_case("true") {
                return 1;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_progress_value() {
        assert_eq!(parse_progress("PROGRESS=75\n"), 75);
        assert_eq!(parse_progress("PROGRESS=100\n"), 100);
    }

    #[test]
    fn control_port_offset() {
        assert_eq!(control_port_for_socks(9050), 9051);
    }
}
