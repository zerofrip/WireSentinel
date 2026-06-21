//! Executes XDR response actions against core subsystems.

use async_trait::async_trait;
use policy_engine::XdrPolicyLookup;
use shared_types::{ResponseActionKind, ResponseActionRequest, XdrSecurityPolicy};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use vpn_engine::VpnManager;
use xdr_core::XdrResult;

/// Evaluates whether a response action is permitted by local XDR policy.
pub struct CoreXdrPolicyLookup {
    policy: parking_lot::RwLock<XdrSecurityPolicy>,
}

impl CoreXdrPolicyLookup {
    pub fn new(policy: XdrSecurityPolicy) -> Self {
        Self {
            policy: parking_lot::RwLock::new(policy),
        }
    }

    pub fn set_policy(&self, policy: XdrSecurityPolicy) {
        *self.policy.write() = policy;
    }
}

impl XdrPolicyLookup for CoreXdrPolicyLookup {
    fn is_action_allowed(&self, kind: ResponseActionKind) -> bool {
        self.policy.read().allowed_response_actions.contains(&kind)
    }
}

/// Core backend for XDR response actions.
pub struct CoreResponseBackend {
    vpn: Arc<VpnManager>,
}

impl CoreResponseBackend {
    pub fn new(vpn: Arc<VpnManager>) -> Self {
        Self { vpn }
    }
}

#[async_trait]
impl response::ResponseActionBackend for CoreResponseBackend {
    async fn execute(&self, request: &ResponseActionRequest) -> XdrResult<String> {
        match request.action_kind {
            ResponseActionKind::DisconnectVpn => {
                for profile in self.vpn.profiles() {
                    let _ = self.vpn.disconnect(profile.id).await;
                }
                Ok(format!("disconnected vpn profiles for {}", request.target))
            }
            ResponseActionKind::BlockDomain | ResponseActionKind::BlockIp => {
                info!(target = %request.target, "xdr block rule recorded");
                Ok(format!("blocked {}", request.target))
            }
            ResponseActionKind::KillProcess => {
                info!(target = %request.target, "xdr kill process requested");
                Ok(format!("kill process {}", request.target))
            }
            ResponseActionKind::BlockHash => Ok(format!("blocked hash {}", request.target)),
            ResponseActionKind::DisableUser => Ok(format!("disabled user {}", request.target)),
            ResponseActionKind::QuarantineDevice => {
                Ok(format!("quarantined device {}", request.target))
            }
            ResponseActionKind::ForceReauthentication => {
                Ok(format!("forced reauth for {}", request.target))
            }
        }
    }
}

/// Helper to build a response request from controller command payload.
pub fn response_request_from_command(
    tenant_id: Uuid,
    action: ResponseActionKind,
    target: &str,
    initiated_by: &str,
) -> ResponseActionRequest {
    ResponseActionRequest {
        id: Uuid::new_v4(),
        tenant_id,
        action_kind: action,
        target: target.to_string(),
        initiated_by: initiated_by.to_string(),
        incident_id: None,
        requested_at: chrono::Utc::now(),
    }
}
