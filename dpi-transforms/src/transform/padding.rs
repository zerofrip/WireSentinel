use super::{TransformContext, TransformModule};
use serde_json::json;

pub struct PaddingTransform;

impl TransformModule for PaddingTransform {
    fn id(&self) -> &'static str {
        "padding"
    }

    fn apply_outbound(&self, ctx: &mut TransformContext) {
        ctx.outbound
            .insert("padding".into(), json!({ "enabled": true, "size": 64 }));
    }

    fn apply_inbound(&self, ctx: &mut TransformContext) {
        ctx.inbound
            .insert("padding".into(), json!({ "enabled": true, "size": 64 }));
    }
}
