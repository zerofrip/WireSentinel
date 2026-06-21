use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared_types::WireSentinelError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XrayProtocol {
    Vless,
    Vmess,
    Trojan,
    Reality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XrayOutboundSpec {
    pub protocol: XrayProtocol,
    pub server: String,
    pub server_port: u16,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub flow: Option<String>,
    #[serde(default)]
    pub sni: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    #[serde(default)]
    pub short_id: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub ws_path: Option<String>,
    #[serde(default)]
    pub ws_host: Option<String>,
}

impl XrayOutboundSpec {
    pub fn from_json(raw: &str) -> Result<Self, WireSentinelError> {
        serde_json::from_str(raw).map_err(WireSentinelError::Serde)
    }
}

pub fn build_config(
    listen_port: u16,
    outbound: &XrayOutboundSpec,
    upstream_socks: Option<&str>,
) -> Value {
    let outbound_value = if let Some(upstream) = upstream_socks {
        let (host, port) = parse_host_port(upstream);
        json!({
            "protocol": "socks",
            "tag": "upstream",
            "settings": {
                "servers": [{ "address": host, "port": port }]
            }
        })
    } else {
        outbound_to_json(outbound)
    };

    json!({
        "log": { "loglevel": "info" },
        "inbounds": [{
            "port": listen_port,
            "listen": "127.0.0.1",
            "protocol": "socks",
            "tag": "socks-in",
            "settings": { "auth": "noauth", "udp": true }
        }],
        "outbounds": [
            outbound_value,
            { "protocol": "freedom", "tag": "direct" }
        ],
        "routing": {
            "rules": [{
                "type": "field",
                "outboundTag": if upstream_socks.is_some() { "upstream" } else { "proxy" }
            }],
            "domainStrategy": "AsIs"
        }
    })
}

fn parse_host_port(addr: &str) -> (String, u16) {
    let mut parts = addr.split(':');
    let host = parts.next().unwrap_or("127.0.0.1").to_string();
    let port = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1080);
    (host, port)
}

fn stream_settings(spec: &XrayOutboundSpec) -> Value {
    if spec.network.as_deref() == Some("ws") {
        return json!({
            "network": "ws",
            "security": if spec.sni.is_some() { "tls" } else { "none" },
            "wsSettings": {
                "path": spec.ws_path.as_deref().unwrap_or("/"),
                "headers": {
                    "Host": spec.ws_host.as_deref().unwrap_or(&spec.server)
                }
            },
            "tlsSettings": {
                "serverName": spec.sni.as_deref().unwrap_or(&spec.server)
            }
        });
    }

    match spec.protocol {
        XrayProtocol::Reality => json!({
            "network": "tcp",
            "security": "reality",
            "realitySettings": {
                "serverName": spec.sni.as_deref().unwrap_or(&spec.server),
                "publicKey": spec.public_key.as_deref().unwrap_or(""),
                "shortId": spec.short_id.as_deref().unwrap_or("")
            }
        }),
        XrayProtocol::Trojan => json!({
            "network": "tcp",
            "security": "tls",
            "tlsSettings": {
                "serverName": spec.sni.as_deref().unwrap_or(&spec.server)
            }
        }),
        _ => json!({
            "network": "tcp",
            "security": "tls",
            "tlsSettings": {
                "serverName": spec.sni.as_deref().unwrap_or(&spec.server)
            }
        }),
    }
}

fn outbound_to_json(spec: &XrayOutboundSpec) -> Value {
    match spec.protocol {
        XrayProtocol::Vless => {
            json!({
                "protocol": "vless",
                "tag": "proxy",
                "settings": {
                    "vnext": [{
                        "address": spec.server,
                        "port": spec.server_port,
                        "users": [{
                            "id": spec.uuid.as_deref().unwrap_or("00000000-0000-0000-0000-000000000000"),
                            "encryption": "none",
                            "flow": spec.flow.as_deref().unwrap_or("")
                        }]
                    }]
                },
                "streamSettings": stream_settings(spec)
            })
        }
        XrayProtocol::Vmess => json!({
            "protocol": "vmess",
            "tag": "proxy",
            "settings": {
                "vnext": [{
                    "address": spec.server,
                    "port": spec.server_port,
                    "users": [{
                        "id": spec.uuid.as_deref().unwrap_or("00000000-0000-0000-0000-000000000000"),
                        "security": "auto"
                    }]
                }]
            }
        }),
        XrayProtocol::Trojan => json!({
            "protocol": "trojan",
            "tag": "proxy",
            "settings": {
                "servers": [{
                    "address": spec.server,
                    "port": spec.server_port,
                    "password": spec.password.as_deref().unwrap_or("")
                }]
            },
            "streamSettings": stream_settings(spec)
        }),
        XrayProtocol::Reality => json!({
            "protocol": "vless",
            "tag": "proxy",
            "settings": {
                "vnext": [{
                    "address": spec.server,
                    "port": spec.server_port,
                    "users": [{
                        "id": spec.uuid.as_deref().unwrap_or("00000000-0000-0000-0000-000000000000"),
                        "encryption": "none",
                        "flow": spec.flow.as_deref().unwrap_or("xtls-rprx-vision")
                    }]
                }]
            },
            "streamSettings": stream_settings(spec)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_reality_config() {
        let spec = XrayOutboundSpec {
            protocol: XrayProtocol::Reality,
            server: "example.com".into(),
            server_port: 443,
            uuid: Some("user-id".into()),
            password: None,
            flow: Some("xtls-rprx-vision".into()),
            sni: Some("www.cloudflare.com".into()),
            public_key: Some("pk".into()),
            short_id: Some("abcd".into()),
            network: None,
            ws_path: None,
            ws_host: None,
        };
        let cfg = build_config(1081, &spec, None);
        assert_eq!(cfg["inbounds"][0]["port"], 1081);
        assert_eq!(cfg["outbounds"][0]["streamSettings"]["security"], "reality");
    }
}
