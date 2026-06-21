use crate::route::TrafficRoute;
use crate::rule::RuleAction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictKind {
    Allow,
    Block,
    RouteViaVpn,
    RouteDirect,
    LogOnly,
    RouteViaTailnet,
    RouteViaTor,
    RouteViaProxy,
    RouteViaProxyChain,
    RouteViaChain,
    RouteAnonymous,
    RouteViaMixnet,
    SegmentDeny,
}

impl From<&TrafficRoute> for VerdictKind {
    fn from(route: &TrafficRoute) -> Self {
        match route {
            TrafficRoute::Direct => Self::RouteDirect,
            TrafficRoute::WireGuard(_) | TrafficRoute::AmneziaWG(_) => Self::RouteViaVpn,
            TrafficRoute::Tailnet(_) => Self::RouteViaTailnet,
            TrafficRoute::Tor(_) => Self::RouteViaTor,
            TrafficRoute::Proxy(_) => Self::RouteViaProxy,
            TrafficRoute::ProxyChain(_) => Self::RouteViaProxyChain,
            TrafficRoute::Chain(_) => Self::RouteViaChain,
            TrafficRoute::Anonymous(_) => Self::RouteAnonymous,
            TrafficRoute::Katzenpost(_)
            | TrafficRoute::Loopix(_)
            | TrafficRoute::FederatedMixnet(_) => Self::RouteViaMixnet,
            TrafficRoute::Blocked => Self::Block,
        }
    }
}

impl From<&RuleAction> for VerdictKind {
    fn from(action: &RuleAction) -> Self {
        match action {
            RuleAction::Allow => Self::Allow,
            RuleAction::Block => Self::Block,
            RuleAction::RouteViaVpn(_) => Self::RouteViaVpn,
            RuleAction::RouteDirect => Self::RouteDirect,
            RuleAction::LogOnly => Self::LogOnly,
            RuleAction::RouteViaTailnet(_) => Self::RouteViaTailnet,
            RuleAction::RouteViaTor(_) => Self::RouteViaTor,
            RuleAction::RouteViaProxy(_) => Self::RouteViaProxy,
            RuleAction::RouteViaProxyChain(_) => Self::RouteViaProxyChain,
            RuleAction::RouteViaChain(_) => Self::RouteViaChain,
            RuleAction::RouteViaAnonymous(_) => Self::RouteAnonymous,
            RuleAction::RouteViaMixnet(_) => Self::RouteViaMixnet,
            RuleAction::SegmentDeny(_) => Self::SegmentDeny,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub kind: VerdictKind,
    pub matched_rule_id: Option<Uuid>,
    pub reason: String,
    pub vpn_profile_id: Option<Uuid>,
}

impl Verdict {
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            kind: VerdictKind::Allow,
            matched_rule_id: None,
            reason: reason.into(),
            vpn_profile_id: None,
        }
    }

    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            kind: VerdictKind::Block,
            matched_rule_id: None,
            reason: reason.into(),
            vpn_profile_id: None,
        }
    }

    pub fn from_route(route: &TrafficRoute, rule_id: Option<Uuid>, reason: impl Into<String>) -> Self {
        Self {
            kind: VerdictKind::from(route),
            matched_rule_id: rule_id,
            reason: reason.into(),
            vpn_profile_id: route.profile_id(),
        }
    }

    pub fn from_rule(action: &RuleAction, rule_id: Uuid, reason: impl Into<String>) -> Self {
        let vpn_profile_id = match action {
            RuleAction::RouteViaVpn(id)
            | RuleAction::RouteViaTailnet(id)
            | RuleAction::RouteViaTor(id)
            | RuleAction::RouteViaProxy(id)
            | RuleAction::RouteViaProxyChain(id)
            | RuleAction::RouteViaChain(id)
            | RuleAction::RouteViaMixnet(id) => Some(*id),
            RuleAction::RouteViaAnonymous(route) => match route {
                crate::AnonymousRoute::Tor(id)
                | crate::AnonymousRoute::TorBridge(id)
                | crate::AnonymousRoute::FutureMixnet(id)
                | crate::AnonymousRoute::Katzenpost(id)
                | crate::AnonymousRoute::Loopix(id) => Some(*id),
                crate::AnonymousRoute::MultiHop(ids) => ids.first().copied(),
                crate::AnonymousRoute::FederatedMixnet { profile_id, .. } => Some(*profile_id),
            },
            _ => None,
        };
        Self {
            kind: VerdictKind::from(action),
            matched_rule_id: Some(rule_id),
            reason: reason.into(),
            vpn_profile_id,
        }
    }
}
