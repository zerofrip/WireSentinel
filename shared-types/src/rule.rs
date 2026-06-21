use crate::AnonymousRoute;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    App(Uuid),
    Domain(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum RuleAction {
    Allow,
    Block,
    RouteViaVpn(Uuid),
    RouteDirect,
    LogOnly,
    RouteViaTailnet(Uuid),
    RouteViaTor(Uuid),
    RouteViaProxy(Uuid),
    RouteViaProxyChain(Uuid),
    RouteViaChain(Uuid),
    RouteViaAnonymous(AnonymousRoute),
    RouteViaMixnet(Uuid),
    SegmentDeny(Uuid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    Blacklist,
    Whitelist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: Uuid,
    /// Higher priority rules are evaluated first.
    pub priority: i32,
    pub scope: RuleScope,
    pub action: RuleAction,
    pub enabled: bool,
    pub description: Option<String>,
}

impl Rule {
    pub fn global(priority: i32, action: RuleAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            priority,
            scope: RuleScope::Global,
            action,
            enabled: true,
            description: None,
        }
    }

    pub fn for_app(app_id: Uuid, priority: i32, action: RuleAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            priority,
            scope: RuleScope::App(app_id),
            action,
            enabled: true,
            description: None,
        }
    }

    pub fn for_domain(domain: impl Into<String>, priority: i32, action: RuleAction) -> Self {
        Self {
            id: Uuid::new_v4(),
            priority,
            scope: RuleScope::Domain(domain.into()),
            action,
            enabled: true,
            description: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruleset {
    pub mode: PolicyMode,
    pub rules: Vec<Rule>,
    pub kill_switch_active: bool,
}

impl Default for Ruleset {
    fn default() -> Self {
        Self {
            mode: PolicyMode::Blacklist,
            rules: Vec::new(),
            kill_switch_active: false,
        }
    }
}
