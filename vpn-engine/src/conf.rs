use shared_types::VpnBackendKind;
use std::collections::HashMap;
use std::path::Path;

/// Parsed WireGuard / AmneziaWG configuration.
#[derive(Debug, Clone, Default)]
pub struct WireGuardConfig {
    pub interface: InterfaceSection,
    pub peers: Vec<PeerSection>,
}

#[derive(Debug, Clone, Default)]
pub struct InterfaceSection {
    pub private_key: Option<String>,
    pub address: Vec<String>,
    pub dns: Vec<String>,
    pub mtu: Option<u16>,
    pub listen_port: Option<u16>,
    // AmneziaWG obfuscation params
    pub jc: Option<u32>,
    pub jmin: Option<u32>,
    pub jmax: Option<u32>,
    pub s1: Option<u32>,
    pub s2: Option<u32>,
    pub s3: Option<u32>,
    pub s4: Option<u32>,
    pub h1: Option<u32>,
    pub h2: Option<u32>,
    pub h3: Option<u32>,
    pub h4: Option<u32>,
    pub i1: Option<String>,
    pub i2: Option<String>,
    pub i3: Option<String>,
    pub i4: Option<String>,
    pub i5: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PeerSection {
    pub public_key: Option<String>,
    pub preshared_key: Option<String>,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u16>,
}

/// Parse a WireGuard .conf file (INI-style).
pub fn parse_conf(content: &str) -> WireGuardConfig {
    let mut config = WireGuardConfig::default();
    let mut current_section: Option<&str> = None;
    let mut current_peer: Option<PeerSection> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(peer) = current_peer.take() {
                config.peers.push(peer);
            }
            current_section = Some(&line[1..line.len() - 1]);
            if current_section == Some("Peer") {
                current_peer = Some(PeerSection::default());
            }
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match current_section {
                Some("Interface") => set_interface_field(&mut config.interface, key, value),
                Some("Peer") => {
                    if let Some(ref mut peer) = current_peer {
                        set_peer_field(peer, key, value);
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(peer) = current_peer {
        config.peers.push(peer);
    }
    config
}

fn set_interface_field(section: &mut InterfaceSection, key: &str, value: &str) {
    match key {
        "PrivateKey" => section.private_key = Some(value.into()),
        "Address" => section
            .address
            .extend(value.split(',').map(|s| s.trim().to_string())),
        "DNS" => section
            .dns
            .extend(value.split(',').map(|s| s.trim().to_string())),
        "MTU" => section.mtu = value.parse().ok(),
        "ListenPort" => section.listen_port = value.parse().ok(),
        "Jc" => section.jc = value.parse().ok(),
        "Jmin" => section.jmin = value.parse().ok(),
        "Jmax" => section.jmax = value.parse().ok(),
        "S1" => section.s1 = value.parse().ok(),
        "S2" => section.s2 = value.parse().ok(),
        "S3" => section.s3 = value.parse().ok(),
        "S4" => section.s4 = value.parse().ok(),
        "H1" => section.h1 = value.parse().ok(),
        "H2" => section.h2 = value.parse().ok(),
        "H3" => section.h3 = value.parse().ok(),
        "H4" => section.h4 = value.parse().ok(),
        "I1" => section.i1 = Some(value.into()),
        "I2" => section.i2 = Some(value.into()),
        "I3" => section.i3 = Some(value.into()),
        "I4" => section.i4 = Some(value.into()),
        "I5" => section.i5 = Some(value.into()),
        _ => {}
    }
}

fn set_peer_field(peer: &mut PeerSection, key: &str, value: &str) {
    match key {
        "PublicKey" => peer.public_key = Some(value.into()),
        "PresharedKey" => peer.preshared_key = Some(value.into()),
        "Endpoint" => peer.endpoint = Some(value.into()),
        "AllowedIPs" => peer
            .allowed_ips
            .extend(value.split(',').map(|s| s.trim().to_string())),
        "PersistentKeepalive" => peer.persistent_keepalive = value.parse().ok(),
        _ => {}
    }
}

/// Detect VPN backend from config content.
pub fn detect_backend(content: &str) -> VpnBackendKind {
    let config = parse_conf(content);
    let has_awg = config.interface.jc.is_some()
        || config.interface.h1.is_some()
        || config.interface.i1.is_some();
    if has_awg {
        VpnBackendKind::AmneziaWg
    } else {
        VpnBackendKind::WireGuardNt
    }
}

/// Serialize config back to .conf format.
pub fn write_conf(config: &WireGuardConfig) -> String {
    let mut out = String::new();
    out.push_str("[Interface]\n");
    if let Some(ref k) = config.interface.private_key {
        out.push_str(&format!("PrivateKey = {k}\n"));
    }
    if !config.interface.address.is_empty() {
        out.push_str(&format!(
            "Address = {}\n",
            config.interface.address.join(", ")
        ));
    }
    if !config.interface.dns.is_empty() {
        out.push_str(&format!("DNS = {}\n", config.interface.dns.join(", ")));
    }
    if let Some(v) = config.interface.mtu {
        out.push_str(&format!("MTU = {v}\n"));
    }
    if let Some(v) = config.interface.listen_port {
        out.push_str(&format!("ListenPort = {v}\n"));
    }
    write_awg_field(&mut out, "Jc", config.interface.jc);
    write_awg_field(&mut out, "Jmin", config.interface.jmin);
    write_awg_field(&mut out, "Jmax", config.interface.jmax);
    write_awg_field(&mut out, "S1", config.interface.s1);
    write_awg_field(&mut out, "S2", config.interface.s2);
    write_awg_field(&mut out, "S3", config.interface.s3);
    write_awg_field(&mut out, "S4", config.interface.s4);
    write_awg_field(&mut out, "H1", config.interface.h1);
    write_awg_field(&mut out, "H2", config.interface.h2);
    write_awg_field(&mut out, "H3", config.interface.h3);
    write_awg_field(&mut out, "H4", config.interface.h4);
    write_awg_str(&mut out, "I1", config.interface.i1.as_deref());
    write_awg_str(&mut out, "I2", config.interface.i2.as_deref());
    write_awg_str(&mut out, "I3", config.interface.i3.as_deref());
    write_awg_str(&mut out, "I4", config.interface.i4.as_deref());
    write_awg_str(&mut out, "I5", config.interface.i5.as_deref());
    for peer in &config.peers {
        out.push_str("\n[Peer]\n");
        if let Some(ref k) = peer.public_key {
            out.push_str(&format!("PublicKey = {k}\n"));
        }
        if let Some(ref ep) = peer.endpoint {
            out.push_str(&format!("Endpoint = {ep}\n"));
        }
        if let Some(ref psk) = peer.preshared_key {
            out.push_str(&format!("PresharedKey = {psk}\n"));
        }
        if !peer.allowed_ips.is_empty() {
            out.push_str(&format!("AllowedIPs = {}\n", peer.allowed_ips.join(", ")));
        }
        if let Some(ka) = peer.persistent_keepalive {
            out.push_str(&format!("PersistentKeepalive = {ka}\n"));
        }
    }
    out
}

/// Encode AmneziaWG config for native tunnel DLL (MVP: INI text bytes).
pub fn encode_awg_config(config: &WireGuardConfig) -> shared_types::Result<Vec<u8>> {
    Ok(write_conf(config).into_bytes())
}

fn write_awg_field(out: &mut String, key: &str, value: Option<u32>) {
    if let Some(v) = value {
        out.push_str(&format!("{key} = {v}\n"));
    }
}

fn write_awg_str(out: &mut String, key: &str, value: Option<&str>) {
    if let Some(v) = value {
        out.push_str(&format!("{key} = {v}\n"));
    }
}

pub fn read_conf_file(path: &Path) -> std::io::Result<WireGuardConfig> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_conf(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_wg() {
        let conf = r#"
[Interface]
PrivateKey = abc
Address = 10.0.0.2/32
DNS = 1.1.1.1

[Peer]
PublicKey = def
Endpoint = 1.2.3.4:51820
AllowedIPs = 0.0.0.0/0
"#;
        let parsed = parse_conf(conf);
        assert_eq!(parsed.interface.private_key.as_deref(), Some("abc"));
        assert_eq!(parsed.peers.len(), 1);
        assert_eq!(detect_backend(conf), VpnBackendKind::WireGuardNt);
    }

    #[test]
    fn detect_amneziawg() {
        let conf = "[Interface]\nPrivateKey = x\nJc = 4\n";
        assert_eq!(detect_backend(conf), VpnBackendKind::AmneziaWg);
    }
}
