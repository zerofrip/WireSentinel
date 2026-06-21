mod doh;
mod dot;
mod doq;

pub use doh::DohProvider;
pub use dot::DotProvider;
pub use doq::DoqProvider;

use shared_types::{DnsTransport, Result, WireSentinelError};
use std::sync::Arc;

use crate::provider::DnsProvider;

pub fn from_record(
    name: &str,
    transport: DnsTransport,
    endpoint: &str,
) -> Result<Arc<dyn DnsProvider>> {
    match transport {
        DnsTransport::Doh => Ok(Arc::new(DohProvider::from_url(name, endpoint)?)),
        DnsTransport::Dot => {
            let (host, port) = parse_dot_endpoint(endpoint)?;
            Ok(Arc::new(DotProvider::new(&host, port)))
        }
        DnsTransport::Doq => Ok(Arc::new(DoqProvider::new(name, endpoint))),
        DnsTransport::Plain => Err(WireSentinelError::Dns(
            "plain DNS upstream is not supported".into(),
        )),
    }
}

pub fn parse_dot_endpoint(url: &str) -> Result<(String, u16)> {
    let trimmed = url.trim();
    let without_scheme = trimmed
        .strip_prefix("tls://")
        .or_else(|| trimmed.strip_prefix("dot://"))
        .unwrap_or(trimmed);

    if let Some((host, port)) = without_scheme.rsplit_once(':') {
        let port = port
            .parse()
            .map_err(|_| WireSentinelError::Dns(format!("invalid DoT port: {port}")))?;
        return Ok((host.to_string(), port));
    }

    Ok((without_scheme.to_string(), 853))
}
