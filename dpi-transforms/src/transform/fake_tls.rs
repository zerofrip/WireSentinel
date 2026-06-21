use super::{TransformContext, TransformModule};
use serde_json::json;

pub struct FakeTlsTransform;

impl TransformModule for FakeTlsTransform {
    fn id(&self) -> &'static str {
        "fake_tls"
    }

    fn apply_outbound(&self, ctx: &mut TransformContext) {
        ctx.outbound.insert(
            "tls".into(),
            json!({
                "enabled": true,
                "utls": { "enabled": true, "fingerprint": "chrome" }
            }),
        );
    }

    fn apply_inbound(&self, ctx: &mut TransformContext) {
        ctx.inbound.insert(
            "tls".into(),
            json!({
                "enabled": true,
                "utls": { "enabled": true, "fingerprint": "chrome" }
            }),
        );
    }
}
