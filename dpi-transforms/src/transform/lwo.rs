use super::{TransformContext, TransformModule};
use serde_json::json;

/// Lightweight WireGuard Obfuscation — minimal padding + fake TLS fingerprint for WG transports.
pub struct LightweightWireGuardObfuscation;

impl TransformModule for LightweightWireGuardObfuscation {
    fn id(&self) -> &'static str {
        "lwo"
    }

    fn apply_outbound(&self, ctx: &mut TransformContext) {
        ctx.outbound.insert(
            "padding".into(),
            json!({ "enabled": true, "size": 32 }),
        );
        ctx.outbound.insert(
            "tls".into(),
            json!({
                "enabled": true,
                "utls": { "enabled": true, "fingerprint": "chrome" }
            }),
        );
        ctx.outbound.insert(
            "lwo".into(),
            json!({ "enabled": true, "mode": "lightweight" }),
        );
    }

    fn apply_inbound(&self, ctx: &mut TransformContext) {
        ctx.inbound.insert(
            "padding".into(),
            json!({ "enabled": true, "size": 32 }),
        );
        ctx.inbound.insert(
            "lwo".into(),
            json!({ "enabled": true, "mode": "lightweight" }),
        );
    }
}
