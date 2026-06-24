from pathlib import Path

p = Path("core-service/src/api/routes.rs")
text = p.read_text()
old_import = """use crate::api::mixnet_routes::{
    create_anonymous_route, delete_anonymous_route, get_anonymous_route,
    get_cover_traffic_settings, get_privacy_analytics, list_anonymous_routes, list_mixnet_profiles,
    mixnet_routes, mixnet_status, start_anonymous_route, start_mixnet, stop_anonymous_route,
    stop_mixnet, update_anonymous_route, update_cover_traffic_settings,
};"""
new_import = old_import + """
use crate::api::anonymity_routes::{
    create_anonymous_service, get_anonymity_entropy, get_anonymity_status,
    get_privacy_anonymity_analytics, list_anonymous_services, list_katzenpost_profiles,
    list_loopix_profiles, simulate_decoy_route, start_katzenpost, start_loopix,
};"""
if "/anonymity/entropy" not in text:
    text = text.replace(old_import, new_import, 1)
    old_route = '        .route("/privacy/analytics", get(get_privacy_analytics))'
    new_route = old_route + """
        .route("/privacy/anonymity", get(get_privacy_anonymity_analytics))
        .route("/anonymity", get(get_anonymity_status))
        .route("/anonymity/entropy", get(get_anonymity_entropy))
        .route(
            "/anonymity/services",
            get(list_anonymous_services).post(create_anonymous_service),
        )
        .route("/anonymity/decoy/simulate", post(simulate_decoy_route))
        .route("/anonymity/katzenpost", get(list_katzenpost_profiles))
        .route("/anonymity/katzenpost/start", post(start_katzenpost))
        .route("/anonymity/loopix", get(list_loopix_profiles))
        .route("/anonymity/loopix/start", post(start_loopix))"""
    text = text.replace(old_route, new_route, 1)
    p.write_text(text)
    print("patched")
else:
    print("already patched")
