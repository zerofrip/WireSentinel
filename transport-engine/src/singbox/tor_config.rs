use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared_types::WireSentinelError;
use std::collections::HashMap;
use std::path::PathBuf;

/// Parameters for sing-box `type: tor` outbound.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorOutboundSpec {
    pub executable_path: PathBuf,
    pub data_directory: PathBuf,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default)]
    pub torrc: HashMap<String, String>,
}

impl TorOutboundSpec {
    pub fn from_json(raw: &str) -> Result<Self, WireSentinelError> {
        serde_json::from_str(raw).map_err(WireSentinelError::Serde)
    }

    pub fn with_upstream_proxy(mut self, upstream_socks: &str) -> Self {
        self.torrc
            .insert("Socks5Proxy".into(), upstream_socks.to_string());
        self
    }
}

fn tor_outbound_json(spec: &TorOutboundSpec) -> Value {
    json!({
        "type": "tor",
        "tag": "proxy",
        "executable_path": spec.executable_path.to_string_lossy(),
        "data_directory": spec.data_directory.to_string_lossy(),
        "extra_args": spec.extra_args,
        "torrc": spec.torrc,
    })
}

/// Builds sing-box JSON: local mixed inbound → tor outbound (optionally chained via torrc Socks5Proxy).
pub fn build_tor_config(
    listen_port: u16,
    tor: &TorOutboundSpec,
    upstream_socks: Option<&str>,
) -> Value {
    let mut spec = tor.clone();
    if let Some(upstream) = upstream_socks {
        spec = spec.with_upstream_proxy(upstream);
    }

    let outbound_value = tor_outbound_json(&spec);

    json!({
        "log": { "level": "info" },
        "inbounds": [{
            "type": "mixed",
            "tag": "mixed-in",
            "listen": "127.0.0.1",
            "listen_port": listen_port
        }],
        "outbounds": [
            outbound_value,
            { "type": "direct", "tag": "direct" }
        ],
        "route": {
            "rules": [{ "outbound": "proxy" }],
            "final": "direct"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tor_config_has_tor_outbound_and_mixed_inbound() {
        let mut torrc = HashMap::new();
        torrc.insert("ClientOnly".into(), "1".into());
        let spec = TorOutboundSpec {
            executable_path: PathBuf::from(r"C:\WireSentinel\tor.exe"),
            data_directory: PathBuf::from(r"C:\ProgramData\WireSentinel\tor\abc"),
            extra_args: vec![],
            torrc,
        };
        let cfg = build_tor_config(9050, &spec, None);
        assert_eq!(cfg["inbounds"][0]["listen_port"], 9050);
        assert_eq!(cfg["outbounds"][0]["type"], "tor");
        assert_eq!(
            cfg["outbounds"][0]["executable_path"],
            r"C:\WireSentinel\tor.exe"
        );
    }

    #[test]
    fn upstream_sets_socks5_proxy_in_torrc() {
        let spec = TorOutboundSpec {
            executable_path: PathBuf::from("tor.exe"),
            data_directory: PathBuf::from("/tmp/tor"),
            extra_args: vec![],
            torrc: HashMap::new(),
        };
        let cfg = build_tor_config(10800, &spec, Some("127.0.0.1:1080"));
        assert_eq!(
            cfg["outbounds"][0]["torrc"]["Socks5Proxy"],
            "127.0.0.1:1080"
        );
    }
}
