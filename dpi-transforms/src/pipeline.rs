//! Preset-driven transform pipelines for transport configs.

use crate::transform::{
    CamouflageTransform, FragmentTransform, JitterTransform, LightweightWireGuardObfuscation,
    PaddingTransform, TransformContext, TransformModule,
};
use serde_json::Value;
use shared_types::ObfuscationPreset;

pub struct TransformPipeline {
    modules: Vec<Box<dyn TransformModule>>,
}

impl TransformPipeline {
    pub fn new(modules: Vec<Box<dyn TransformModule>>) -> Self {
        Self { modules }
    }

    pub fn empty() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn from_preset(preset: ObfuscationPreset) -> Self {
        let modules: Vec<Box<dyn TransformModule>> = match preset {
            ObfuscationPreset::Disabled => Vec::new(),
            ObfuscationPreset::Basic => vec![Box::new(PaddingTransform)],
            ObfuscationPreset::Balanced => vec![
                Box::new(PaddingTransform),
                Box::new(JitterTransform),
            ],
            ObfuscationPreset::Aggressive => vec![
                Box::new(FragmentTransform),
                Box::new(PaddingTransform),
                Box::new(JitterTransform),
                Box::new(CamouflageTransform),
            ],
            ObfuscationPreset::Lwo => vec![Box::new(LightweightWireGuardObfuscation)],
            ObfuscationPreset::Socks5Handshake => Vec::new(),
        };
        Self { modules }
    }

    pub fn module_ids(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.id()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn apply_outbound(&self, ctx: &mut TransformContext) {
        for module in &self.modules {
            module.apply_outbound(ctx);
        }
    }

    pub fn apply_inbound(&self, ctx: &mut TransformContext) {
        for module in &self.modules {
            module.apply_inbound(ctx);
        }
    }

    /// Apply all pipeline modules to sing-box / xray-style transport JSON configs.
    pub fn apply_to_config(&self, config: &mut Value) {
        if let Some(outbounds) = config.get_mut("outbounds").and_then(|v| v.as_array_mut()) {
            for outbound in outbounds {
                if let Some(obj) = outbound.as_object_mut() {
                    let mut ctx = TransformContext::new();
                    self.apply_outbound(&mut ctx);
                    ctx.merge_outbound(obj);
                }
            }
        }
        if let Some(inbounds) = config.get_mut("inbounds").and_then(|v| v.as_array_mut()) {
            for inbound in inbounds {
                if let Some(obj) = inbound.as_object_mut() {
                    let mut ctx = TransformContext::new();
                    self.apply_inbound(&mut ctx);
                    ctx.merge_inbound(obj);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lwo_preset_includes_lwo_module() {
        assert_eq!(
            TransformPipeline::from_preset(ObfuscationPreset::Lwo).module_ids(),
            vec!["lwo"]
        );
    }

    #[test]
    fn preset_module_counts() {
        assert!(TransformPipeline::from_preset(ObfuscationPreset::Disabled).is_empty());
        assert_eq!(
            TransformPipeline::from_preset(ObfuscationPreset::Basic).module_ids(),
            vec!["padding"]
        );
        assert_eq!(
            TransformPipeline::from_preset(ObfuscationPreset::Balanced).module_ids(),
            vec!["padding", "jitter"]
        );
        assert_eq!(
            TransformPipeline::from_preset(ObfuscationPreset::Aggressive).module_ids(),
            vec!["fragment", "padding", "jitter", "camouflage"]
        );
    }

    #[test]
    fn applies_padding_to_outbound() {
        let pipeline = TransformPipeline::from_preset(ObfuscationPreset::Basic);
        let mut config = json!({
            "outbounds": [{ "type": "vless", "tag": "proxy" }],
            "inbounds": [{ "type": "mixed", "tag": "in" }]
        });
        pipeline.apply_to_config(&mut config);
        assert_eq!(config["outbounds"][0]["padding"]["enabled"], true);
        assert_eq!(config["inbounds"][0]["padding"]["enabled"], true);
    }

    #[test]
    fn aggressive_adds_fragment_and_camouflage() {
        let pipeline = TransformPipeline::from_preset(ObfuscationPreset::Aggressive);
        let mut config = json!({
            "outbounds": [{ "type": "socks", "tag": "proxy" }]
        });
        pipeline.apply_to_config(&mut config);
        let outbound = &config["outbounds"][0];
        assert_eq!(outbound["fragment"]["enabled"], true);
        assert_eq!(outbound["padding"]["enabled"], true);
        assert_eq!(outbound["jitter"]["enabled"], true);
        assert_eq!(outbound["camouflage"]["enabled"], true);
    }

    #[test]
    fn disabled_leaves_config_unchanged() {
        let pipeline = TransformPipeline::from_preset(ObfuscationPreset::Disabled);
        let mut config = json!({ "outbounds": [{ "type": "direct" }] });
        let before = config.clone();
        pipeline.apply_to_config(&mut config);
        assert_eq!(config, before);
    }
}
