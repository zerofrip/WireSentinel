use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared_types::WireSentinelError;

/// Supported sing-box outbound / inbound protocol kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SingBoxProtocol {
    Socks,
    Http,
    Shadowsocks,
    Vless,
    Vmess,
    Trojan,
}

/// User-supplied outbound parameters parsed from `TransportProfile.config_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingBoxOutboundSpec {
    pub protocol: SingBoxProtocol,
    pub server: String,
    pub server_port: u16,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub flow: Option<String>,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub sni: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub ws_path: Option<String>,
    #[serde(default)]
    pub ws_host: Option<String>,
}

impl SingBoxOutboundSpec {
    pub fn from_json(raw: &str) -> Result<Self, WireSentinelError> {
        serde_json::from_str(raw).map_err(WireSentinelError::Serde)
    }
}

/// Builds sing-box JSON configuration with a local mixed inbound and one outbound.
pub fn build_config(
    listen_port: u16,
    outbound: &SingBoxOutboundSpec,
    upstream_socks: Option<&str>,
) -> Value {
    let outbound_value = if let Some(upstream) = upstream_socks {
        json!({
            "type": "socks",
            "tag": "upstream",
            "server": upstream.split(':').next().unwrap_or("127.0.0.1"),
            "server_port": upstream
                .split(':')
                .nth(1)
                .and_then(|p| p.parse().ok())
                .unwrap_or(1080),
            "version": "5"
        })
    } else {
        outbound_to_json(outbound)
    };

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
            "rules": [{ "outbound": if upstream_socks.is_some() { "upstream" } else { "proxy" } }],
            "final": "direct"
        }
    })
}

fn outbound_to_json(spec: &SingBoxOutboundSpec) -> Value {
    match spec.protocol {
        SingBoxProtocol::Socks => json!({
            "type": "socks",
            "tag": "proxy",
            "server": spec.server,
            "server_port": spec.server_port,
            "version": "5"
        }),
        SingBoxProtocol::Http => json!({
            "type": "http",
            "tag": "proxy",
            "server": spec.server,
            "server_port": spec.server_port
        }),
        SingBoxProtocol::Shadowsocks => json!({
            "type": "shadowsocks",
            "tag": "proxy",
            "server": spec.server,
            "server_port": spec.server_port,
            "method": spec.method.as_deref().unwrap_or("2022-blake3-aes-128-gcm"),
            "password": spec.password.as_deref().unwrap_or("")
        }),
        SingBoxProtocol::Vless => {
            let mut obj = json!({
                "type": "vless",
                "tag": "proxy",
                "server": spec.server,
                "server_port": spec.server_port,
                "uuid": spec.uuid.as_deref().unwrap_or("00000000-0000-0000-0000-000000000000")
            });
            if let Some(flow) = &spec.flow {
                obj["flow"] = json!(flow);
            }
            if spec.tls {
                obj["tls"] = json!({
                    "enabled": true,
                    "server_name": spec.sni.as_deref().unwrap_or(&spec.server)
                });
            }
            if spec.network.as_deref() == Some("ws") {
                obj["transport"] = json!({
                    "type": "ws",
                    "path": spec.ws_path.as_deref().unwrap_or("/"),
                    "headers": {
                        "Host": spec.ws_host.as_deref().unwrap_or(&spec.server)
                    }
                });
            }
            obj
        }
        SingBoxProtocol::Vmess => json!({
            "type": "vmess",
            "tag": "proxy",
            "server": spec.server,
            "server_port": spec.server_port,
            "uuid": spec.uuid.as_deref().unwrap_or("00000000-0000-0000-0000-000000000000"),
            "security": "auto"
        }),
        SingBoxProtocol::Trojan => json!({
            "type": "trojan",
            "tag": "proxy",
            "server": spec.server,
            "server_port": spec.server_port,
            "password": spec.password.as_deref().unwrap_or(""),
            "tls": {
                "enabled": true,
                "server_name": spec.sni.as_deref().unwrap_or(&spec.server)
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_vless_config() {
        let spec = SingBoxOutboundSpec {
            protocol: SingBoxProtocol::Vless,
            server: "example.com".into(),
            server_port: 443,
            uuid: Some("abc".into()),
            password: None,
            method: None,
            flow: Some("xtls-rprx-vision".into()),
            tls: true,
            sni: Some("example.com".into()),
            network: None,
            ws_path: None,
            ws_host: None,
        };
        let cfg = build_config(1080, &spec, None);
        assert_eq!(cfg["inbounds"][0]["listen_port"], 1080);
        assert_eq!(cfg["outbounds"][0]["type"], "vless");
    }
}
