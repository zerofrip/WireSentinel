use super::{TransformContext, TransformModule};
use serde_json::json;

pub struct JitterTransform;

impl TransformModule for JitterTransform {
    fn id(&self) -> &'static str {
        "jitter"
    }

    fn apply_outbound(&self, ctx: &mut TransformContext) {
        ctx.outbound.insert(
            "jitter".into(),
            json!({ "enabled": true, "min_ms": 1, "max_ms": 10 }),
        );
    }

    fn apply_inbound(&self, ctx: &mut TransformContext) {
        ctx.inbound.insert(
            "jitter".into(),
            json!({ "enabled": true, "min_ms": 1, "max_ms": 10 }),
        );
    }
}
