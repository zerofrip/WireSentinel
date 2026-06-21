use crate::phase7::AnonymousRoute;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Routing decision for network traffic (split-tunnel ready).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum TrafficRoute {
    Direct,
    WireGuard(Uuid),
    AmneziaWG(Uuid),
    Blocked,
    Tailnet(Uuid),
    Tor(Uuid),
    Anonymous(AnonymousRoute),
    Proxy(Uuid),
    ProxyChain(Uuid),
    Chain(Uuid),
    Katzenpost(Uuid),
    Loopix(Uuid),
    FederatedMixnet(Uuid),
}

impl TrafficRoute {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    pub fn profile_id(&self) -> Option<Uuid> {
        match self {
            Self::WireGuard(id) | Self::AmneziaWG(id) | Self::Tailnet(id) | Self::Tor(id) => {
                Some(*id)
            }
            Self::Proxy(id)
            | Self::ProxyChain(id)
            | Self::Chain(id)
            | Self::Katzenpost(id)
            | Self::Loopix(id)
            | Self::FederatedMixnet(id) => Some(*id),
            Self::Anonymous(AnonymousRoute::Tor(id) | AnonymousRoute::TorBridge(id)) => Some(*id),
            Self::Anonymous(AnonymousRoute::FutureMixnet(id)) => Some(*id),
            Self::Anonymous(AnonymousRoute::Katzenpost(id) | AnonymousRoute::Loopix(id)) => {
                Some(*id)
            }
            Self::Anonymous(AnonymousRoute::FederatedMixnet { profile_id, .. }) => {
                Some(*profile_id)
            }
            _ => None,
        }
    }

    pub fn is_vpn(&self) -> bool {
        matches!(
            self,
            Self::WireGuard(_) | Self::AmneziaWG(_) | Self::Tailnet(_)
        )
    }

    pub fn requires_transport(&self) -> bool {
        matches!(
            self,
            Self::Tor(_)
                | Self::Anonymous(_)
                | Self::Proxy(_)
                | Self::ProxyChain(_)
                | Self::Chain(_)
                | Self::Katzenpost(_)
                | Self::Loopix(_)
                | Self::FederatedMixnet(_)
        )
    }
}
