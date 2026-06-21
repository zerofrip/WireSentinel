//! DPI evasion transform modules operating on transport JSON configs.

mod camouflage;
mod fake_tls;
mod fragment;
mod jitter;
mod lwo;
mod padding;

pub use camouflage::CamouflageTransform;
pub use fake_tls::FakeTlsTransform;
pub use fragment::FragmentTransform;
pub use jitter::JitterTransform;
pub use lwo::LightweightWireGuardObfuscation;
pub use padding::PaddingTransform;

use serde_json::{Map, Value};

/// Mutable outbound/inbound JSON config maps for transform application.
#[derive(Debug, Clone, Default)]
pub struct TransformContext {
    pub outbound: Map<String, Value>,
    pub inbound: Map<String, Value>,
}

impl TransformContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge_outbound(&mut self, obj: &mut Map<String, Value>) {
        let outbound = std::mem::take(&mut self.outbound);
        for (k, v) in outbound {
            obj.insert(k, v);
        }
    }

    pub fn merge_inbound(&mut self, obj: &mut Map<String, Value>) {
        let inbound = std::mem::take(&mut self.inbound);
        for (k, v) in inbound {
            obj.insert(k, v);
        }
    }
}

/// A single DPI evasion transform applied to transport configs.
pub trait TransformModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn apply_outbound(&self, ctx: &mut TransformContext);
    fn apply_inbound(&self, ctx: &mut TransformContext);
}
