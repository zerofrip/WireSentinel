//! Apply handshake proxy settings to WireGuard configs before connect.

use crate::conf::WireGuardConfig;
use crate::obfuscation::{PreparedHandshake, Socks5HandshakeBackend};
use chrono::Utc;
use event_bus::EventBus;
use shared_types::{Result, ServiceEventInner, VPNProfile, WireSentinelError};

/// Active handshake proxy session tied to a VPN profile connect attempt.
pub struct HandshakeProxySession {
    pub prepared: PreparedHandshake,
}

/// If enabled on profile, rewrite peer endpoint to local SOCKS5 relay for handshake.
pub async fn apply_handshake_proxy(
    profile: &VPNProfile,
    config: &mut WireGuardConfig,
    events: Option<&EventBus>,
) -> Result<Option<HandshakeProxySession>> {
    let settings = match profile.handshake_proxy.as_ref().filter(|s| s.enabled) {
        Some(s) if !s.host.is_empty() => s.clone(),
        _ => return Ok(None),
    };

    let endpoint = config
        .peers
        .first()
        .and_then(|p| p.endpoint.clone())
        .ok_or_else(|| WireSentinelError::Vpn("handshake proxy: peer endpoint missing".into()))?;

    let backend = Socks5HandshakeBackend::new(settings.clone());
    match backend.prepare_handshake_endpoint(&endpoint).await {
        Ok(prepared) => {
            if let Some(events) = events {
                events.publish(
                    ServiceEventInner::HandshakeProxyConnected {
                        profile_id: profile.id,
                        proxy_host: settings.host.clone(),
                        proxy_port: settings.port,
                    }
                    .with_timestamp(Utc::now()),
                );
            }
            if let Some(peer) = config.peers.first_mut() {
                peer.endpoint = Some(prepared.relay_endpoint.clone());
            }
            Ok(Some(HandshakeProxySession { prepared }))
        }
        Err(e) => {
            if let Some(events) = events {
                events.publish(
                    ServiceEventInner::HandshakeProxyFailed {
                        profile_id: profile.id,
                        error: e.to_string(),
                    }
                    .with_timestamp(Utc::now()),
                );
            }
            Err(e)
        }
    }
}
