use super::traits::{Result, VpnProfileRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{HandshakeProxySettings, TransportKind, VPNProfile, VpnBackendKind, WireSentinelError};
use sqlx::SqlitePool;
use std::path::PathBuf;
use uuid::Uuid;

pub struct SqliteVpnProfileRepository {
    pool: SqlitePool,
}

impl SqliteVpnProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn backend_str(b: VpnBackendKind) -> &'static str {
    match b {
        VpnBackendKind::WireGuardNt => "wireguard_nt",
        VpnBackendKind::AmneziaWg => "amnezia_wg",
        VpnBackendKind::Tailscale => "tailscale",
    }
}

fn backend_from_str(s: &str) -> Result<VpnBackendKind> {
    match s {
        "wireguard_nt" => Ok(VpnBackendKind::WireGuardNt),
        "amnezia_wg" => Ok(VpnBackendKind::AmneziaWg),
        "tailscale" => Ok(VpnBackendKind::Tailscale),
        other => Err(WireSentinelError::Config(format!("unknown backend: {other}"))),
    }
}

fn transport_kind_str(k: TransportKind) -> &'static str {
    match k {
        TransportKind::Direct => "direct",
        TransportKind::WireGuard => "wire_guard",
        TransportKind::AmneziaWg => "amnezia_wg",
        TransportKind::SingBox => "sing_box",
        TransportKind::Xray => "xray",
        TransportKind::Tor => "tor",
        TransportKind::TlsTunnel => "tls_tunnel",
        TransportKind::WebSocketTunnel => "ws_tunnel",
        TransportKind::Mixnet => "mixnet",
        TransportKind::Proxy => "proxy",
    }
}

fn transport_kind_from_str(s: &str) -> Result<TransportKind> {
    match s {
        "direct" => Ok(TransportKind::Direct),
        "wire_guard" => Ok(TransportKind::WireGuard),
        "amnezia_wg" => Ok(TransportKind::AmneziaWg),
        "sing_box" => Ok(TransportKind::SingBox),
        "xray" => Ok(TransportKind::Xray),
        "tor" => Ok(TransportKind::Tor),
        "tls_tunnel" => Ok(TransportKind::TlsTunnel),
        "ws_tunnel" => Ok(TransportKind::WebSocketTunnel),
        "mixnet" => Ok(TransportKind::Mixnet),
        "proxy" => Ok(TransportKind::Proxy),
        other => Err(WireSentinelError::Config(format!("unknown transport kind: {other}"))),
    }
}

#[async_trait]
impl VpnProfileRepository for SqliteVpnProfileRepository {
    async fn list(&self) -> Result<Vec<VPNProfile>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                i32,
                String,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT id, name, backend, auto_connect, created_at, group_name, transport_kind, chain_id, obfuscation_profile_id, handshake_proxy_json FROM vpn_profiles ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(
                    id,
                    name,
                    backend,
                    auto_connect,
                    created_at,
                    _group,
                    transport_kind,
                    chain_id,
                    obfuscation_profile_id,
                    handshake_proxy_json,
                )| {
                    let handshake_proxy = handshake_proxy_json
                        .map(|j| serde_json::from_str::<HandshakeProxySettings>(&j))
                        .transpose()
                        .map_err(WireSentinelError::Serde)?;
                    Ok(VPNProfile {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        name,
                        backend: backend_from_str(&backend)?,
                        config_path: PathBuf::from(format!("db://{id}")),
                        auto_connect: auto_connect != 0,
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                        transport_kind: transport_kind_from_str(&transport_kind)?,
                        chain_id: chain_id
                            .map(|s| Uuid::parse_str(&s))
                            .transpose()
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        obfuscation_profile_id: obfuscation_profile_id
                            .map(|s| Uuid::parse_str(&s))
                            .transpose()
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        handshake_proxy,
                    })
                },
            )
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<VPNProfile>> {
        Ok(self.list().await?.into_iter().find(|p| p.id == id))
    }

    async fn get_config_blob(&self, id: Uuid) -> Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT config_blob FROM vpn_profiles WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn insert(&self, profile: &VPNProfile, config_blob: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT INTO vpn_profiles (id, name, backend, config_blob, auto_connect, group_name, created_at, transport_kind, chain_id, obfuscation_profile_id, handshake_proxy_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(backend_str(profile.backend))
        .bind(config_blob)
        .bind(profile.auto_connect as i32)
        .bind(None::<String>)
        .bind(profile.created_at.to_rfc3339())
        .bind(transport_kind_str(profile.transport_kind))
        .bind(profile.chain_id.map(|id| id.to_string()))
        .bind(profile.obfuscation_profile_id.map(|id| id.to_string()))
        .bind(
            profile
                .handshake_proxy
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(WireSentinelError::Serde)?,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &VPNProfile, config_blob: Option<&[u8]>) -> Result<()> {
        if let Some(blob) = config_blob {
            sqlx::query(
                "UPDATE vpn_profiles SET name = ?, backend = ?, config_blob = ?, auto_connect = ?, transport_kind = ?, chain_id = ?, obfuscation_profile_id = ?, handshake_proxy_json = ? WHERE id = ?",
            )
            .bind(&profile.name)
            .bind(backend_str(profile.backend))
            .bind(blob)
            .bind(profile.auto_connect as i32)
            .bind(transport_kind_str(profile.transport_kind))
            .bind(profile.chain_id.map(|id| id.to_string()))
            .bind(profile.obfuscation_profile_id.map(|id| id.to_string()))
            .bind(
                profile
                    .handshake_proxy
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(WireSentinelError::Serde)?,
            )
            .bind(profile.id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        } else {
            sqlx::query(
                "UPDATE vpn_profiles SET name = ?, backend = ?, auto_connect = ?, transport_kind = ?, chain_id = ?, obfuscation_profile_id = ?, handshake_proxy_json = ? WHERE id = ?",
            )
            .bind(&profile.name)
            .bind(backend_str(profile.backend))
            .bind(profile.auto_connect as i32)
            .bind(transport_kind_str(profile.transport_kind))
            .bind(profile.chain_id.map(|id| id.to_string()))
            .bind(profile.obfuscation_profile_id.map(|id| id.to_string()))
            .bind(
                profile
                    .handshake_proxy
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(WireSentinelError::Serde)?,
            )
            .bind(profile.id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM vpn_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
