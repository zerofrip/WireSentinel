use super::{TransformContext, TransformModule};
use serde_json::json;

pub struct FragmentTransform;

impl TransformModule for FragmentTransform {
    fn id(&self) -> &'static str {
        "fragment"
    }

    fn apply_outbound(&self, ctx: &mut TransformContext) {
        ctx.outbound.insert(
            "fragment".into(),
            json!({ "enabled": true, "length": 100, "interval": 10 }),
        );
    }

    fn apply_inbound(&self, ctx: &mut TransformContext) {
        ctx.inbound.insert(
            "fragment".into(),
            json!({ "enabled": true, "length": 100, "interval": 10 }),
        );
    }
}
