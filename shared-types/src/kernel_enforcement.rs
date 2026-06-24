use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::DriverState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementBackend {
    #[default]
    Signed,
    CustomKernel,
}

impl EnforcementBackend {
    pub fn parse(s: &str) -> Self {
        match s {
            "custom_kernel" => Self::CustomKernel,
            _ => Self::Signed,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Signed => "signed",
            Self::CustomKernel => "custom_kernel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum GuardianMode {
    #[default]
    Wfp,
    Ndis,
    Hybrid,
}

impl GuardianMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "ndis" => Self::Ndis,
            "hybrid" => Self::Hybrid,
            _ => Self::Wfp,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wfp => "wfp",
            Self::Ndis => "ndis",
            Self::Hybrid => "hybrid",
        }
    }

    pub fn uses_ndis(self) -> bool {
        matches!(self, Self::Ndis | Self::Hybrid)
    }
}

/// Resolved low-level driver settings derived from [`EnforcementBackend`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnforcementMapping {
    pub backend: EnforcementBackend,
    pub guardian_mode: GuardianMode,
    pub wfp_engine_impl: &'static str,
    pub use_windivert: bool,
}

impl EnforcementMapping {
    pub fn from_backend(backend: EnforcementBackend) -> Self {
        match backend {
            EnforcementBackend::Signed => Self {
                backend,
                guardian_mode: GuardianMode::Wfp,
                wfp_engine_impl: "userspace",
                use_windivert: true,
            },
            EnforcementBackend::CustomKernel => Self {
                backend,
                guardian_mode: GuardianMode::Hybrid,
                wfp_engine_impl: "kernel",
                use_windivert: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct EnforcementSettingsResponse {
    pub enforcement_backend: EnforcementBackend,
    pub guardian_mode: GuardianMode,
    pub wfp_engine_impl: String,
    pub components: EnforcementComponentsHealth,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct EnforcementComponentsHealth {
    pub wfp: String,
    pub wireguard: String,
    pub windivert: String,
    pub singbox: String,
    pub guardian: String,
    pub ndis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SetEnforcementBackendRequest {
    pub enforcement_backend: EnforcementBackend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct NdisHealth {
    pub available: bool,
    pub state: String,
    pub filter_attached: bool,
    pub active_route_count: u32,
    pub active_redirect_count: u32,
    pub classify_count: u64,
    pub error_count: u64,
    pub message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct KernelTelemetryV2 {
    pub guardian_mode: GuardianMode,
    pub guardian: Option<DriverState>,
    pub ndis: Option<NdisHealth>,
    pub classify_count: u64,
    pub redirect_count: u64,
    pub transform_count: u64,
    pub cover_traffic_count: u64,
    pub error_count: u64,
    pub dropped_count: u64,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct KernelStatistics {
    pub guardian_mode: GuardianMode,
    pub wfp_filter_count: u32,
    pub ndis_route_count: u32,
    pub ndis_redirect_count: u32,
    pub telemetry_events: u64,
    pub security_violations: u64,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransformModuleKind {
    Padding,
    Jitter,
    Fragment,
    Camouflage,
    Lwo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TransformModule {
    pub kind: TransformModuleKind,
    pub parameter0: u32,
    pub parameter1: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum KernelObfuscationPreset {
    #[default]
    Disabled,
    Basic,
    Balanced,
    Aggressive,
    Lwo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TransformProfile {
    pub preset: KernelObfuscationPreset,
    pub modules: Vec<TransformModule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum KernelCoverTrafficMode {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct KernelCoverTrafficProfile {
    pub mode: KernelCoverTrafficMode,
    pub min_interval_ms: u32,
    pub max_interval_ms: u32,
    pub min_payload_bytes: u32,
    pub max_payload_bytes: u32,
    pub burst_count: u32,
    pub enabled: bool,
}

impl Default for KernelCoverTrafficProfile {
    fn default() -> Self {
        Self {
            mode: KernelCoverTrafficMode::Off,
            min_interval_ms: 0,
            max_interval_ms: 0,
            min_payload_bytes: 0,
            max_payload_bytes: 0,
            burst_count: 0,
            enabled: false,
        }
    }
}
