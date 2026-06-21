#!/usr/bin/env python3
from pathlib import Path

ROOT = Path("/home/zero/github/WireSentinel")

def patch(path, old, new, label):
    p = ROOT / path
    text = p.read_text()
    if new.strip() in text:
        print(f"skip {label}")
        return
    if old not in text:
        raise SystemExit(f"missing anchor for {label}")
    p.write_text(text.replace(old, new, 1))
    print(f"patched {label}")

# deps.rs imports
patch(
    "core-service/src/deps.rs",
    "use crate::mixnet_redirect::MixnetRedirectEngine;",
    """use crate::anonymity::AnonymityService;
use crate::anonymity_decoy::AnonymityDecoyService;
use crate::anonymity_discovery::AnonymityDiscoveryService;
use crate::anonymity_entropy::RouteEntropyBridge;
use crate::anonymity_security::AnonymitySecurityPolicy;
use crate::mixnet_redirect::MixnetRedirectEngine;""",
    "deps imports",
)

patch(
    "core-service/src/deps.rs",
    "    pub mixnet_redirect: Arc<MixnetRedirectEngine>,\n    pub api_token: String,",
    """    pub mixnet_redirect: Arc<MixnetRedirectEngine>,
    pub anonymity_security: Arc<AnonymitySecurityPolicy>,
    pub anonymity: Arc<AnonymityService>,
    pub anonymity_entropy: Arc<RouteEntropyBridge>,
    pub anonymity_discovery: Arc<AnonymityDiscoveryService>,
    pub anonymity_decoy: Arc<AnonymityDecoyService>,
    pub api_token: String,""",
    "deps struct",
)

patch(
    "core-service/src/deps.rs",
    """        let mixnet_redirect = Arc::new(MixnetRedirectEngine::new(
            Arc::clone(&kernel_route_bridge),
            Arc::clone(&listen_ports),
        ));
        let kernel_telemetry = Arc::new(KernelTelemetryService::new(""",
    """        let mixnet_redirect = Arc::new(MixnetRedirectEngine::new(
            Arc::clone(&kernel_route_bridge),
            Arc::clone(&listen_ports),
        ));
        let anonymity_security = Arc::new(AnonymitySecurityPolicy::new(events.clone()));
        let anonymity_manager = Arc::new(anonymity_core::AnonymityManager::new());
        let anonymity = Arc::new(AnonymityService::new(
            Arc::clone(&storage),
            events.clone(),
            anonymity_manager,
            Arc::clone(&listen_ports),
            Arc::clone(&anonymity_security),
        ));
        let anonymity_entropy = Arc::new(RouteEntropyBridge::new(events.clone()));
        let anonymity_discovery = Arc::new(AnonymityDiscoveryService::new());
        let anonymity_decoy = Arc::new(AnonymityDecoyService::new(
            anonymity_security.as_ref(),
            events.clone(),
        ));
        let kernel_telemetry = Arc::new(KernelTelemetryService::new(""",
    "deps build services",
)

patch(
    "core-service/src/deps.rs",
    """            .with_anonymous_routing(Arc::clone(&anonymous_routing))
            .with_kernel_route_bridge(Arc::clone(&kernel_route_bridge))
            .with_proxy_redirect(Arc::clone(&proxy_redirect))
            .with_mixnet_redirect(Arc::clone(&mixnet_redirect)),""",
    """            .with_anonymous_routing(Arc::clone(&anonymous_routing))
            .with_anonymity(Arc::clone(&anonymity))
            .with_kernel_route_bridge(Arc::clone(&kernel_route_bridge))
            .with_proxy_redirect(Arc::clone(&proxy_redirect))
            .with_mixnet_redirect(Arc::clone(&mixnet_redirect)),""",
    "deps split tunnel",
)

patch(
    "core-service/src/deps.rs",
    """            mixnet_redirect,
            api_token,""",
    """            mixnet_redirect,
            anonymity_security,
            anonymity,
            anonymity_entropy,
            anonymity_discovery,
            anonymity_decoy,
            api_token,""",
    "deps ok struct",
)

patch(
    "core-service/src/deps.rs",
    """        let privacy_analytics = Arc::new(PrivacyAnalyticsService::new(
            Arc::clone(&storage),
            events.clone(),
            Arc::clone(&mixnet),
            Arc::clone(&cover_traffic),
        ));""",
    """        let privacy_analytics = Arc::new(PrivacyAnalyticsService::new(
            Arc::clone(&storage),
            events.clone(),
            Arc::clone(&mixnet),
            Arc::clone(&cover_traffic),
            Arc::clone(&anonymity),
            anonymity_entropy.clone(),
        ));""",
    "privacy analytics ctor",
)

print("done")
