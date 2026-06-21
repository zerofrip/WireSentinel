use super::{TransformContext, TransformModule};
use serde_json::json;

pub struct CamouflageTransform;

impl TransformModule for CamouflageTransform {
    fn id(&self) -> &'static str {
        "camouflage"
    }

    fn apply_outbound(&self, ctx: &mut TransformContext) {
        ctx.outbound.insert(
            "camouflage".into(),
            json!({ "enabled": true, "pattern": "http" }),
        );
    }

    fn apply_inbound(&self, ctx: &mut TransformContext) {
        ctx.inbound.insert(
            "camouflage".into(),
            json!({ "enabled": true, "pattern": "http" }),
        );
    }
}
