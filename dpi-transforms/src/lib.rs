//! Modular DPI evasion transform hooks (best-effort, user-controlled).

mod pipeline;
pub mod transform;

pub use pipeline::TransformPipeline;
pub use transform::{
    CamouflageTransform, FakeTlsTransform, FragmentTransform, JitterTransform, PaddingTransform,
    TransformContext, TransformModule,
};

// Legacy registry API (transport-engine integration tests).
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformKind {
    TlsObfuscation,
    TcpFallback,
    PacketFragmentation,
    ExternalModule { name: String, config_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    pub kind: TransformKind,
    pub enabled: bool,
}

#[async_trait]
pub trait LegacyTransformModule: Send + Sync {
    fn name(&self) -> &str;
    async fn apply(&self, target: &str) -> bool;
}

pub struct SingBoxModule;
pub struct XrayModule;

#[async_trait]
impl LegacyTransformModule for SingBoxModule {
    fn name(&self) -> &str {
        "sing-box"
    }
    async fn apply(&self, _target: &str) -> bool {
        false
    }
}

#[async_trait]
impl LegacyTransformModule for XrayModule {
    fn name(&self) -> &str {
        "xray-core"
    }
    async fn apply(&self, _target: &str) -> bool {
        false
    }
}

/// Deprecated alias for [`LegacyTransformModule`].
pub use LegacyTransformModule as TransformModuleHook;

#[derive(Default)]
pub struct TransformRegistry {
    transforms: Vec<TransformConfig>,
    modules: Vec<Box<dyn LegacyTransformModule>>,
}

impl TransformRegistry {
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
            modules: Vec::new(),
        }
    }

    pub fn register(&mut self, config: TransformConfig) {
        self.transforms.push(config);
    }

    pub fn register_module(&mut self, module: Box<dyn LegacyTransformModule>) {
        self.modules.push(module);
    }

    pub fn active(&self) -> Vec<&TransformConfig> {
        self.transforms.iter().filter(|t| t.enabled).collect()
    }
}
